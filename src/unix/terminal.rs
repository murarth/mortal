use std::convert::TryFrom;
use std::fs::File;
use std::io;
use std::mem::{replace, zeroed};
use std::os::unix::io::{FromRawFd, IntoRawFd, RawFd};
use std::path::Path;
use std::str::from_utf8;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{LockResult, Mutex, MutexGuard, TryLockResult};
use std::time::Duration;

use libc::{
    ioctl,
    c_int, c_ushort, termios,
    STDIN_FILENO, STDOUT_FILENO, STDERR_FILENO, TIOCGWINSZ,
};

use nix::errno::Errno;
use nix::sys::select::{select, FdSet};
use nix::sys::signal::{
    sigaction,
    SaFlags, SigAction, SigHandler, Signal as NixSignal, SigSet,
};
use nix::sys::termios::{
    tcgetattr, tcsetattr,
    SetArg, InputFlags, LocalFlags,
};
use nix::sys::time::{TimeVal, TimeValLike};
use nix::unistd::{read, write};

use smallstr::SmallString;

use terminfo::{self, capability as cap, Database};
use terminfo::capability::Expansion;
use terminfo::expand::Context;

use crate::priv_util::{map_lock_result, map_try_lock_result};
use crate::sequence::{FindResult, SequenceMap};
use crate::signal::{Signal, SignalSet};
use crate::terminal::{
    Color, Cursor, CursorMode, Event, Key, PrepareConfig, Size, Style, Theme,
    MouseButton, MouseEvent, MouseInput, ModifierState,
};
use crate::util::prefixes;

const OUT_BUFFER_SIZE: usize = 8192;

const XTERM_ENABLE_MOUSE: &str = "\x1b[?1006h\x1b[?1002h";
const XTERM_DISABLE_MOUSE: &str = "\x1b[?1006l\x1b[?1002l";
const XTERM_ENABLE_MOUSE_MOTION: &str = "\x1b[?1003h";
const XTERM_DISABLE_MOUSE_MOTION: &str = "\x1b[?1003l";
const XTERM_MOUSE_INTRO: &str = "\x1b[<";

const XTERM_SHIFT_MASK: u32 = 0x04;
const XTERM_META_MASK: u32  = 0x08;
const XTERM_CTRL_MASK: u32  = 0x10;
const XTERM_MODIFIER_MASK: u32 = XTERM_SHIFT_MASK | XTERM_META_MASK | XTERM_CTRL_MASK;

type SeqMap = SequenceMap<SmallString<[u8; 8]>, SeqData>;

#[derive(Copy, Clone)]
enum SeqData {
    XTermMouse,
    Key(Key),
}

pub struct Terminal {
    info: Database,
    out_fd: RawFd,
    in_fd: RawFd,
    owned_fd: bool,
    sequences: SeqMap,
    reader: Mutex<Reader>,
    writer: Mutex<Writer>,
}

pub struct TerminalReadGuard<'a> {
    term: &'a Terminal,
    reader: MutexGuard<'a, Reader>,
}

pub struct TerminalWriteGuard<'a> {
    term: &'a Terminal,
    writer: MutexGuard<'a, Writer>,
}

struct Reader {
    in_buffer: Vec<u8>,
    resume: Option<Resume>,
    report_signals: SignalSet,
}

struct Writer {
    context: Context,
    out_buffer: Vec<u8>,
    fg: Option<Color>,
    bg: Option<Color>,
    cur_style: Style,
}

impl Terminal {
    fn new(in_fd: RawFd, out_fd: RawFd, owned_fd: bool) -> io::Result<Terminal> {
        let info = Database::from_env().map_err(ti_to_io)?;
        let sequences = sequences(&info);

        Ok(Terminal{
            info,
            in_fd,
            out_fd,
            owned_fd,
            sequences,
            reader: Mutex::new(Reader{
                in_buffer: Vec::new(),
                resume: None,
                report_signals: SignalSet::new(),
            }),
            writer: Mutex::new(Writer::new()),
        })
    }

    pub fn open<P: AsRef<Path>>(path: P) -> io::Result<Terminal> {
        let fd = open_rw(path)?;

        let r = Terminal::new(fd, fd, true);

        if r.is_err() {
            unsafe { close_fd(fd); }
        }

        r
    }

    pub fn stdout() -> io::Result<Terminal> {
        Terminal::new(STDIN_FILENO, STDOUT_FILENO, false)
    }

    pub fn stderr() -> io::Result<Terminal> {
        Terminal::new(STDIN_FILENO, STDERR_FILENO, false)
    }

    pub fn name(&self) -> &str {
        self.info.name()
    }

    fn is_xterm(&self) -> bool {
        is_xterm(self.name())
    }

    pub fn size(&self) -> io::Result<Size> {
        self.lock_writer().size()
    }

    pub fn wait_event(&self, timeout: Option<Duration>) -> io::Result<bool> {
        self.lock_reader().wait_event(timeout)
    }

    pub fn read_event(&self, timeout: Option<Duration>) -> io::Result<Option<Event>> {
        self.lock_reader().read_event(timeout)
    }

    pub fn read_raw(&self, buf: &mut [u8], timeout: Option<Duration>) -> io::Result<Option<Event>> {
        self.lock_reader().read_raw(buf, timeout)
    }

    pub fn enter_screen(&self) -> io::Result<()> {
        self.lock_writer().enter_screen()
    }

    pub fn exit_screen(&self) -> io::Result<()> {
        self.lock_writer().exit_screen()
    }

    pub fn prepare(&self, config: PrepareConfig) -> io::Result<PrepareState> {
        self.lock_reader().prepare(config)
    }

    pub fn restore(&self, state: PrepareState) -> io::Result<()> {
        self.lock_reader().restore(state)
    }

    pub fn clear_screen(&self) -> io::Result<()> {
        self.lock_writer().clear_screen()
    }

    pub fn clear_to_line_end(&self) -> io::Result<()> {
        self.lock_writer().clear_to_line_end()
    }

    pub fn clear_to_screen_end(&self) -> io::Result<()> {
        self.lock_writer().clear_to_screen_end()
    }

    pub fn move_up(&self, n: usize) -> io::Result<()> {
        if n != 0 {
            self.lock_writer().move_up(n)?;
        }
        Ok(())
    }

    pub fn move_down(&self, n: usize) -> io::Result<()> {
        if n != 0 {
            self.lock_writer().move_down(n)?;
        }
        Ok(())
    }

    pub fn move_left(&self, n: usize) -> io::Result<()> {
        if n != 0 {
            self.lock_writer().move_left(n)?;
        }
        Ok(())
    }

    pub fn move_right(&self, n: usize) -> io::Result<()> {
        if n != 0 {
            self.lock_writer().move_right(n)?;
        }
        Ok(())
    }

    pub fn move_to_first_column(&self) -> io::Result<()> {
        self.lock_writer().move_to_first_column()
    }

    pub fn set_cursor_mode(&self, mode: CursorMode) -> io::Result<()> {
        self.lock_writer().set_cursor_mode(mode)
    }

    pub fn write_char(&self, ch: char) -> io::Result<()> {
        self.write_str(ch.encode_utf8(&mut [0; 4]))
    }

    pub fn write_str(&self, s: &str) -> io::Result<()> {
        self.lock_writer().write_str(s)
    }

    pub fn write_styled(&self,
            fg: Option<Color>, bg: Option<Color>, style: Style, text: &str)
            -> io::Result<()> {
        self.lock_writer().write_styled(fg, bg, style, text)
    }

    pub fn clear_attributes(&self) -> io::Result<()> {
        self.lock_writer().clear_attributes()
    }

    pub fn set_fg(&self, fg: Option<Color>) -> io::Result<()> {
        self.lock_writer().set_fg(fg)
    }

    pub fn set_bg(&self, bg: Option<Color>) -> io::Result<()> {
        self.lock_writer().set_bg(bg)
    }

    pub fn add_style(&self, style: Style) -> io::Result<()> {
        self.lock_writer().add_style(style)
    }

    pub fn remove_style(&self, style: Style) -> io::Result<()> {
        self.lock_writer().remove_style(style)
    }

    pub fn set_style(&self, style: Style) -> io::Result<()> {
        self.lock_writer().set_style(style)
    }

    pub fn set_theme(&self, theme: Theme) -> io::Result<()> {
        self.lock_writer().set_theme(theme)
    }

    pub fn lock_read(&self) -> LockResult<TerminalReadGuard> {
        map_lock_result(self.reader.lock(),
            |r| TerminalReadGuard::new(self, r))
    }

    pub fn lock_write(&self) -> LockResult<TerminalWriteGuard> {
        map_lock_result(self.writer.lock(),
            |w| TerminalWriteGuard::new(self, w))
    }

    pub fn try_lock_read(&self) -> TryLockResult<TerminalReadGuard> {
        map_try_lock_result(self.reader.try_lock(),
            |r| TerminalReadGuard::new(self, r))
    }

    pub fn try_lock_write(&self) -> TryLockResult<TerminalWriteGuard> {
        map_try_lock_result(self.writer.try_lock(),
            |w| TerminalWriteGuard::new(self, w))
    }

    fn lock_reader(&self) -> TerminalReadGuard {
        self.lock_read().expect("Terminal::lock_reader")
    }

    fn lock_writer(&self) -> TerminalWriteGuard {
        self.lock_write().expect("Terminal::lock_writer")
    }
}

impl Drop for Terminal {
    fn drop(&mut self) {
        if let Err(e) = self.set_cursor_mode(CursorMode::Normal) {
            eprintln!("failed to restore terminal: {}", e);
        }

        if self.owned_fd {
            unsafe { close_fd(self.out_fd); }
        }
    }
}

impl<'a> TerminalReadGuard<'a> {
    fn new(term: &'a Terminal, reader: MutexGuard<'a, Reader>) -> TerminalReadGuard<'a> {
        TerminalReadGuard{term, reader}
    }

    pub fn prepare(&mut self, config: PrepareConfig) -> io::Result<PrepareState> {
        let mut writer = self.term.lock_writer();
        self.prepare_with_lock(&mut writer, config)
    }

    pub fn prepare_with_lock(&mut self, writer: &mut TerminalWriteGuard,
            config: PrepareConfig) -> io::Result<PrepareState> {
        use nix::sys::termios::SpecialCharacterIndices::*;

        let old_tio = tcgetattr(self.term.in_fd).map_err(nix_to_io)?;
        let mut tio = old_tio.clone();

        let mut state = PrepareState{
            old_tio: old_tio.into(),
            old_sigcont: None,
            old_sigint: None,
            old_sigtstp: None,
            old_sigquit: None,
            old_sigwinch: None,
            restore_keypad: false,
            restore_mouse: false,
            prev_resume: self.reader.resume,
        };

        tio.input_flags.remove(
            // Disable carriage return/line feed conversion
            InputFlags::INLCR | InputFlags::ICRNL
        );

        tio.local_flags.remove(
            // Disable canonical mode;
            // this gives us input without waiting for newline or EOF
            // and disables line-editing, treating such inputs as characters.
            // Disable ECHO, preventing input from being written to output.
            LocalFlags::ICANON | LocalFlags::ECHO
        );

        // ISIG, when enabled, causes the process to receive signals when
        // Ctrl-C, Ctrl-\, etc. are input
        if config.block_signals {
            tio.local_flags.remove(LocalFlags::ISIG);
        } else {
            tio.local_flags.insert(LocalFlags::ISIG);
        }

        // IXON, when enabled, allows Ctrl-S/Ctrl-Q to suspend and restart inputs
        if config.enable_control_flow {
            tio.input_flags.insert(InputFlags::IXON);
        } else {
            tio.input_flags.remove(InputFlags::IXON);
        }

        // Allow a read to return with 0 characters ready
        tio.control_chars[VMIN as usize] = 0;
        // Allow a read to return after 0 deciseconds
        tio.control_chars[VTIME as usize] = 0;

        tcsetattr(self.term.in_fd, SetArg::TCSANOW, &tio).map_err(nix_to_io)?;

        if config.enable_mouse {
            if writer.enable_mouse(config.always_track_motion)? {
                state.restore_mouse = true;
            }
        }

        if config.enable_keypad {
            if writer.enable_keypad()? {
                state.restore_keypad = true;
            }
        }

        writer.flush()?;

        let action = SigAction::new(SigHandler::Handler(handle_signal),
            SaFlags::empty(), SigSet::all());

        // Continue and Resize are always handled by the internals,
        // but only reported if requested.
        state.old_sigcont = Some(unsafe { sigaction(NixSignal::SIGCONT, &action).map_err(nix_to_io)? });
        state.old_sigwinch = Some(unsafe { sigaction(NixSignal::SIGWINCH, &action).map_err(nix_to_io)? });

        if config.report_signals.contains(Signal::Interrupt) {
            state.old_sigint = Some(unsafe { sigaction(NixSignal::SIGINT, &action).map_err(nix_to_io)? });
        }
        if config.report_signals.contains(Signal::Suspend) {
            state.old_sigtstp = Some(unsafe { sigaction(NixSignal::SIGTSTP, &action).map_err(nix_to_io)? });
        }
        if config.report_signals.contains(Signal::Quit) {
            state.old_sigquit = Some(unsafe { sigaction(NixSignal::SIGQUIT, &action).map_err(nix_to_io)? });
        }

        self.reader.report_signals = config.report_signals;
        self.reader.resume = Some(Resume{config});

        Ok(state)
    }

    pub fn restore(&mut self, state: PrepareState) -> io::Result<()> {
        let mut writer = self.term.lock_writer();
        self.restore_with_lock(&mut writer, state)
    }

    pub fn restore_with_lock(&mut self, writer: &mut TerminalWriteGuard,
            state: PrepareState) -> io::Result<()> {
        self.reader.resume = state.prev_resume;

        if state.restore_mouse {
            writer.disable_mouse()?;
        }

        if state.restore_keypad {
            writer.disable_keypad()?;
        }

        writer.flush()?;

        tcsetattr(self.term.in_fd, SetArg::TCSANOW, &state.old_tio.into()).map_err(nix_to_io)?;

        unsafe {
            if let Some(ref old) = state.old_sigcont {
                sigaction(NixSignal::SIGCONT, old).map_err(nix_to_io)?;
            }
            if let Some(ref old) = state.old_sigint {
                sigaction(NixSignal::SIGINT, old).map_err(nix_to_io)?;
            }
            if let Some(ref old) = state.old_sigtstp {
                sigaction(NixSignal::SIGTSTP, old).map_err(nix_to_io)?;
            }
            if let Some(ref old) = state.old_sigquit {
                sigaction(NixSignal::SIGQUIT, old).map_err(nix_to_io)?;
            }
            if let Some(ref old) = state.old_sigwinch {
                sigaction(NixSignal::SIGWINCH, old).map_err(nix_to_io)?;
            }
        }

        Ok(())
    }

    pub fn wait_event(&mut self, timeout: Option<Duration>) -> io::Result<bool> {
        if get_signal().is_some() {
            return Ok(true);
        }

        if peek_event(&self.reader.in_buffer, &self.term.sequences)?.is_some() {
            return Ok(true);
        }

        let mut timeout = timeout.map(to_timeval);

        let n = loop {
            let in_fd = self.term.in_fd;

            let mut r_fds = FdSet::new();
            r_fds.insert(in_fd);

            // FIXME: FdSet does not implement Copy or Clone
            let mut e_fds = FdSet::new();
            e_fds.insert(in_fd);

            match select(in_fd + 1,
                    Some(&mut r_fds), None, Some(&mut e_fds), timeout.as_mut()) {
                Ok(n) => break n,
                Err(Errno::EINTR) =>
                    if get_signal().is_some() {
                        return Ok(true);
                    }
                
                Err(e) => return Err(nix_to_io(e))
            }
        };

        Ok(n != 0)
    }

    pub fn read_event(&mut self, timeout: Option<Duration>) -> io::Result<Option<Event>> {
        if let Some(ev) = self.try_read()? {
            return Ok(Some(ev));
        }

        match self.read_into_buffer(timeout)? {
            Some(Event::Raw(_)) => self.try_read(),
            Some(Event::Signal(sig)) => {
                if let Some(ev) = self.handle_signal(sig)? {
                    Ok(Some(ev))
                } else {
                    Ok(None)
                }
            }
            r => Ok(r)
        }
    }

    pub fn read_raw(&mut self, buf: &mut [u8], timeout: Option<Duration>) -> io::Result<Option<Event>> {
        if !self.reader.in_buffer.is_empty() {
            let n = buf.len().min(self.reader.in_buffer.len());
            buf[..n].copy_from_slice(&self.reader.in_buffer[..n]);

            let _ = self.reader.in_buffer.drain(..n);

            return Ok(Some(Event::Raw(n)));
        }

        match self.read_input(buf, timeout)? {
            Some(Event::Signal(sig)) => {
                if let Some(event) = self.handle_signal(sig)? {
                    Ok(Some(event))
                } else {
                    Ok(None)
                }
            }
            r => Ok(r)
        }
    }

    fn read_into_buffer(&mut self, timeout: Option<Duration>) -> io::Result<Option<Event>> {
        // Temporarily replace the buffer to prevent borrow errors
        let mut buf = replace(&mut self.reader.in_buffer, Vec::new());

        buf.reserve(128);

        let len = buf.len();
        let cap = buf.capacity();
        let r;

        unsafe {
            buf.set_len(cap);

            r = self.read_input(&mut buf[len..], timeout);

            match r {
                Ok(Some(Event::Raw(n))) => buf.set_len(len + n),
                _ => buf.set_len(len)
            }
        }

        // Restore the buffer before returning
        self.reader.in_buffer = buf;

        r
    }

    fn read_input(&mut self, buf: &mut [u8], timeout: Option<Duration>) -> io::Result<Option<Event>> {
        // Check for a signal that may have already arrived.
        if let Some(sig) = take_signal() {
            return Ok(Some(Event::Signal(sig)));
        }

        if !self.wait_event(timeout)? {
            return Ok(None);
        }

        // Check for a signal again after waiting
        if let Some(sig) = take_signal() {
            return Ok(Some(Event::Signal(sig)));
        }

        loop {
            match read(self.term.in_fd, buf) {
                Ok(n) => break Ok(Some(Event::Raw(n))),
                Err(Errno::EINTR) => {
                    if let Some(sig) = take_signal() {
                        return Ok(Some(Event::Signal(sig)));
                    }
                }
                Err(e) => return Err(nix_to_io(e))
            }
        }
    }

    fn try_read(&mut self) -> io::Result<Option<Event>> {
        let in_buffer = &mut self.reader.in_buffer;

        if in_buffer.is_empty() {
            Ok(None)
        } else {
            match peek_event(&in_buffer, &self.term.sequences) {
                Ok(Some((ev, n))) => {
                    let _ = in_buffer.drain(..n);
                    Ok(Some(ev))
                }
                Ok(None) => Ok(None),
                Err(e) => Err(e)
            }
        }
    }

    fn handle_signal(&mut self, sig: Signal) -> io::Result<Option<Event>> {
        match sig {
            Signal::Continue => {
                self.resume()?;
            }
            Signal::Resize => {
                let size = self.term.size()?;
                return Ok(Some(Event::Resize(size)));
            }
            _ => ()
        }

        if self.reader.report_signals.contains(sig) {
            Ok(Some(Event::Signal(sig)))
        } else {
            Ok(None)
        }
    }

    fn resume(&mut self) -> io::Result<()> {
        if let Some(resume) = self.reader.resume {
            let _ = self.prepare(resume.config)?;
        }
        Ok(())
    }
}

macro_rules! expand_opt {
    ( $slf:expr , $cap:path ) => { {
        if let Some(cap) = $slf.term.info.get::<$cap>() {
            $slf.expand(cap.expand())
        } else {
            Ok(())
        }
    } };
    ( $slf:expr , $cap:path , |$ex:ident| $expansion:expr ) => { {
        if let Some(cap) = $slf.term.info.get::<$cap>() {
            let $ex = cap.expand();
            $slf.expand($expansion)
        } else {
            Ok(())
        }
    } }
}

macro_rules! expand_req {
    ( $slf:expr , $cap:path , $name:expr ) => { {
        $slf.term.info.get::<$cap>()
            .ok_or_else(|| not_supported($name))
            .and_then(|cap| $slf.expand(cap.expand()))
    } };
    ( $slf:expr , $cap:path , $name:expr , |$ex:ident| $expansion:expr ) => { {
        $slf.term.info.get::<$cap>()
            .ok_or_else(|| not_supported($name))
            .and_then(|cap| {
                let $ex = cap.expand();
                $slf.expand($expansion)
            })
    } }
}

impl<'a> TerminalWriteGuard<'a> {
    fn new(term: &'a Terminal, writer: MutexGuard<'a, Writer>) -> TerminalWriteGuard<'a> {
        TerminalWriteGuard{term, writer}
    }

    pub fn size(&self) -> io::Result<Size> {
        get_winsize(self.term.out_fd)
    }

    fn disable_keypad(&mut self) -> io::Result<()> {
        if let Some(local) = self.term.info.get::<cap::KeypadLocal>() {
            self.expand(local.expand())?;
        }
        Ok(())
    }

    fn enable_keypad(&mut self) -> io::Result<bool> {
        if let Some(xmit) = self.term.info.get::<cap::KeypadXmit>() {
            self.expand(xmit.expand())?;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    fn disable_mouse(&mut self) -> io::Result<()> {
        self.write_bytes(XTERM_DISABLE_MOUSE.as_bytes())?;
        self.write_bytes(XTERM_DISABLE_MOUSE_MOTION.as_bytes())
    }

    fn enable_mouse(&mut self, track_motion: bool) -> io::Result<bool> {
        if self.term.is_xterm() {
            self.write_bytes(XTERM_ENABLE_MOUSE.as_bytes())?;
            if track_motion {
                self.write_bytes(XTERM_ENABLE_MOUSE_MOTION.as_bytes())?;
            }
            Ok(true)
        } else {
            Ok(false)
        }
    }

    fn enter_screen(&mut self) -> io::Result<()> {
        match (self.term.info.get::<cap::EnterCaMode>(),
                self.term.info.get::<cap::ChangeScrollRegion>(),
                self.term.info.get::<cap::CursorHome>()) {
            (enter, Some(scroll), Some(home)) => {
                let size = self.size()?;

                if let Some(enter) = enter {
                    self.expand(enter.expand())?;
                }

                self.expand(scroll.expand()
                    .parameters(0, to_u32(size.lines - 1)))?;
                self.expand(home.expand())?;
            }
            (_, None, _) => return Err(not_supported("change_scroll_region")),
            (_, _, None) => return Err(not_supported("cursor_home")),
        }

        self.clear_attributes()?;
        self.clear_screen()?;

        Ok(())
    }

    fn exit_screen(&mut self) -> io::Result<()> {
        if let Some(exit) = self.term.info.get::<cap::ExitCaMode>() {
            self.expand(exit.expand())?;
            self.flush()?;
        }

        Ok(())
    }

    pub fn clear_attributes(&mut self) -> io::Result<()> {
        if self.writer.fg.is_some() || self.writer.bg.is_some() ||
                !self.writer.cur_style.is_empty() {
            self.writer.fg = None;
            self.writer.bg = None;
            self.writer.cur_style = Style::empty();
            expand_opt!(self, cap::ExitAttributeMode)?;
        }

        Ok(())
    }

    pub fn set_fg(&mut self, fg: Option<Color>) -> io::Result<()> {
        if self.writer.fg == fg {
            Ok(())
        } else {
            if let Some(fg) = fg {
                self.set_fg_color(fg)?;
            } else {
                self.clear_fg()?;
            }

            self.writer.fg = fg;
            Ok(())
        }
    }

    pub fn set_bg(&mut self, bg: Option<Color>) -> io::Result<()> {
        if self.writer.bg == bg {
            Ok(())
        } else {
            if let Some(bg) = bg {
                self.set_bg_color(bg)?;
            } else {
                self.clear_bg()?;
            }

            self.writer.bg = bg;
            Ok(())
        }
    }

    pub fn add_style(&mut self, style: Style) -> io::Result<()> {
        let add = style - self.writer.cur_style;

        if add.contains(Style::BOLD) {
            expand_opt!(self, cap::EnterBoldMode)?;
        }
        if add.contains(Style::ITALIC) {
            expand_opt!(self, cap::EnterItalicsMode)?;
        }
        if add.contains(Style::REVERSE) {
            expand_opt!(self, cap::EnterReverseMode)?;
        }
        if add.contains(Style::UNDERLINE) {
            expand_opt!(self, cap::EnterUnderlineMode)?;
        }

        self.writer.cur_style |= add;

        Ok(())
    }

    pub fn remove_style(&mut self, style: Style) -> io::Result<()> {
        let remove = style & self.writer.cur_style;

        if remove.intersects(Style::BOLD | Style::REVERSE) {
            // terminfo does not contain entries to remove bold or reverse.
            // Instead, we must reset all attributes.
            let new_style = self.writer.cur_style - remove;
            let fg = self.writer.fg;
            let bg = self.writer.bg;
            self.clear_attributes()?;
            self.add_style(new_style)?;
            self.set_fg(fg)?;
            self.set_bg(bg)?;
        } else {
            if remove.contains(Style::ITALIC) {
                expand_opt!(self, cap::ExitItalicsMode)?;
            }
            if remove.contains(Style::UNDERLINE) {
                expand_opt!(self, cap::ExitUnderlineMode)?;
            }

            self.writer.cur_style -= remove;
        }

        Ok(())
    }

    pub fn set_style(&mut self, style: Style) -> io::Result<()> {
        let add = style - self.writer.cur_style;
        let remove = self.writer.cur_style - style;

        if remove.intersects(Style::BOLD | Style::REVERSE) {
            // terminfo does not contain entries to remove bold or reverse.
            // Instead, we must reset all attributes.
            let fg = self.writer.fg;
            let bg = self.writer.bg;
            self.clear_attributes()?;
            self.set_fg(fg)?;
            self.set_bg(bg)?;
            self.add_style(style)?;
        } else {
            self.add_style(add)?;
            self.remove_style(remove)?;
        }

        Ok(())
    }

    pub fn set_theme(&mut self, theme: Theme) -> io::Result<()> {
        self.set_attrs(theme.fg, theme.bg, theme.style)
    }

    pub fn set_attrs(&mut self, fg: Option<Color>, bg: Option<Color>, style: Style) -> io::Result<()> {
        if (self.writer.fg.is_some() && fg.is_none()) ||
                (self.writer.bg.is_some() && bg.is_none()) {
            self.clear_attributes()?;
        }

        self.set_style(style)?;
        self.set_fg(fg)?;
        self.set_bg(bg)?;

        Ok(())
    }

    fn clear_fg(&mut self) -> io::Result<()> {
        let bg = self.writer.bg;
        let style = self.writer.cur_style;

        self.clear_attributes()?;
        self.set_bg(bg)?;
        self.set_style(style)
    }

    fn clear_bg(&mut self) -> io::Result<()> {
        let fg = self.writer.fg;
        let style = self.writer.cur_style;

        self.clear_attributes()?;
        self.set_fg(fg)?;
        self.set_style(style)
    }

    fn set_fg_color(&mut self, fg: Color) -> io::Result<()> {
        expand_opt!(self, cap::SetAForeground,
            |ex| ex.parameters(color_code(fg)))
    }

    fn set_bg_color(&mut self, bg: Color) -> io::Result<()> {
        expand_opt!(self, cap::SetABackground,
            |ex| ex.parameters(color_code(bg)))
    }

    pub fn clear_screen(&mut self) -> io::Result<()> {
        expand_req!(self, cap::ClearScreen, "clear_screen")
    }

    pub fn clear_to_line_end(&mut self) -> io::Result<()> {
        expand_req!(self, cap::ClrEol, "clr_eol")
    }

    pub fn clear_to_screen_end(&mut self) -> io::Result<()> {
        expand_req!(self, cap::ClrEos, "clr_eos")
    }

    pub fn move_up(&mut self, n: usize) -> io::Result<()> {
        if n == 1 {
            expand_req!(self, cap::CursorUp, "cursor_up")?;
        } else if n != 0 {
            expand_req!(self, cap::ParmUpCursor, "parm_cursor_up",
                |ex| ex.parameters(to_u32(n)))?;
        }
        Ok(())
    }

    pub fn move_down(&mut self, n: usize) -> io::Result<()> {
        // Always use ParmDownCursor because CursorDown does not behave
        // as expected outside EnterCaMode state.
        if n != 0 {
            expand_req!(self, cap::ParmDownCursor, "parm_cursor_down",
                |ex| ex.parameters(to_u32(n)))?;
        }
        Ok(())
    }

    pub fn move_left(&mut self, n: usize) -> io::Result<()> {
        if n == 1 {
            expand_req!(self, cap::CursorLeft, "cursor_left")?;
        } else if n != 0 {
            expand_req!(self, cap::ParmLeftCursor, "parm_cursor_left",
                |ex| ex.parameters(to_u32(n)))?;
        }
        Ok(())
    }

    pub fn move_right(&mut self, n: usize) -> io::Result<()> {
        if n == 1 {
            expand_req!(self, cap::CursorRight, "cursor_right")?;
        } else if n != 0 {
            expand_req!(self, cap::ParmRightCursor, "parm_cursor_right",
                |ex| ex.parameters(to_u32(n)))?;
        }
        Ok(())
    }

    pub fn move_to_first_column(&mut self) -> io::Result<()> {
        self.write_bytes(b"\r")
    }

    pub fn move_cursor(&mut self, pos: Cursor) -> io::Result<()> {
        match (self.term.info.get::<cap::CursorAddress>(),
                self.term.info.get::<cap::CursorHome>()) {
            (_, Some(ref home)) if pos == Cursor::default() => {
                self.expand(home.expand())?;
            }
            (Some(addr), _) => {
                self.expand(addr.expand()
                    .parameters(to_u32(pos.line), to_u32(pos.column)))?;
            }
            (None, _) => return Err(not_supported("cursor_address"))
        }

        Ok(())
    }

    pub fn set_cursor_mode(&mut self, mode: CursorMode) -> io::Result<()> {
        match mode {
            CursorMode::Normal | CursorMode::Overwrite => {
                // Overwrite is not supported by Unix terminals.
                // We set to normal in this case as it will reverse
                // a setting of Invisible
                expand_opt!(self, cap::CursorNormal)?;
            }
            CursorMode::Invisible => {
                expand_opt!(self, cap::CursorInvisible)?;
            }
        }

        Ok(())
    }

    pub fn write_char(&mut self, ch: char) -> io::Result<()> {
        self.write_str(ch.encode_utf8(&mut [0; 4]))
    }

    pub fn write_str(&mut self, s: &str) -> io::Result<()> {
        self.write_bytes(s.as_bytes())
    }

    pub fn write_styled(&mut self,
            fg: Option<Color>, bg: Option<Color>, style: Style, text: &str)
            -> io::Result<()> {
        self.set_attrs(fg, bg, style)?;

        self.write_str(text)?;
        self.clear_attributes()
    }

    fn write_bytes(&mut self, buf: &[u8]) -> io::Result<()> {
        if buf.len() + self.writer.out_buffer.len() > self.writer.out_buffer.capacity() {
            self.flush()?;
        }

        if buf.len() > self.writer.out_buffer.capacity() {
            self.write_data(buf).1
        } else {
            self.writer.out_buffer.extend(buf);
            Ok(())
        }
    }

    pub fn flush(&mut self) -> io::Result<()> {
        let (n, res) = self.write_data(&self.writer.out_buffer);
        self.writer.out_buffer.drain(..n);
        res
    }

    fn write_data(&self, buf: &[u8]) -> (usize, io::Result<()>) {
        let mut offset = 0;

        let r = loop {
            if offset == buf.len() {
                break Ok(());
            }

            match write(self.term.out_fd, buf) {
                Ok(0) => break Err(io::Error::from(io::ErrorKind::WriteZero)),
                Ok(n) => offset += n,
                Err(Errno::EINTR) => continue,
                Err(e) => break Err(nix_to_io(e))
            }
        };

        (offset, r)
    }

    fn expand<T: AsRef<[u8]>>(&mut self, exp: Expansion<T>) -> io::Result<()> {
        let writer = &mut *self.writer;
        exp
            .with(&mut writer.context)
            .to(&mut writer.out_buffer)
            .map_err(ti_to_io)
    }
}

impl<'a> Drop for TerminalWriteGuard<'a> {
    fn drop(&mut self) {
        if let Err(e) = self.flush() {
            eprintln!("failed to flush terminal: {}", e);
        }
    }
}

impl Writer {
    fn new() -> Writer {
        Writer{
            context: Context::default(),
            out_buffer: Vec::with_capacity(OUT_BUFFER_SIZE),
            fg: None,
            bg: None,
            cur_style: Style::empty(),
        }
    }
}

fn is_xterm(name: &str) -> bool {
    // Includes such terminal names as "xterm-256color"
    name == "xterm" || name.starts_with("xterm-")
}

fn sequences(info: &Database) -> SeqMap {
    let mut sequences = SequenceMap::new();

    macro_rules! add {
        ( $seq:ty , $key:expr ) => { {
            if let Some(seq) = info.get::<$seq>() {
                if let Some(s) = ascii_str(seq.as_ref()) {
                    sequences.insert(s.into(), SeqData::Key($key));
                }
            }
        } }
    }

    add!(cap::KeyUp,        Key::Up);
    add!(cap::KeyDown,      Key::Down);
    add!(cap::KeyLeft,      Key::Left);
    add!(cap::KeyRight,     Key::Right);
    add!(cap::KeyHome,      Key::Home);
    add!(cap::KeyEnd,       Key::End);
    add!(cap::KeyNPage,     Key::PageDown);
    add!(cap::KeyPPage,     Key::PageUp);
    add!(cap::KeyDc,        Key::Delete);
    add!(cap::KeyIc,        Key::Insert);
    add!(cap::KeyF1,        Key::F(1));
    add!(cap::KeyF2,        Key::F(2));
    add!(cap::KeyF3,        Key::F(3));
    add!(cap::KeyF4,        Key::F(4));
    add!(cap::KeyF5,        Key::F(5));
    add!(cap::KeyF6,        Key::F(6));
    add!(cap::KeyF7,        Key::F(7));
    add!(cap::KeyF8,        Key::F(8));
    add!(cap::KeyF9,        Key::F(9));
    add!(cap::KeyF10,       Key::F(10));
    add!(cap::KeyF11,       Key::F(11));
    add!(cap::KeyF12,       Key::F(12));

    if is_xterm(info.name()) {
        sequences.insert(XTERM_MOUSE_INTRO.into(), SeqData::XTermMouse);
    }

    sequences
}

pub struct PrepareState {
    old_tio: termios,
    old_sigcont: Option<SigAction>,
    old_sigint: Option<SigAction>,
    old_sigtstp: Option<SigAction>,
    old_sigquit: Option<SigAction>,
    old_sigwinch: Option<SigAction>,
    restore_keypad: bool,
    restore_mouse: bool,
    prev_resume: Option<Resume>,
}

#[derive(Copy, Clone, Debug)]
struct Resume {
    config: PrepareConfig,
}

unsafe fn close_fd(fd: RawFd) {
    drop(File::from_raw_fd(fd));
}

fn open_rw<P: AsRef<Path>>(path: P) -> io::Result<RawFd> {
    use std::fs::OpenOptions;

    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .open(path)?;

    Ok(file.into_raw_fd())
}

#[repr(C)]
struct Winsize {
    ws_row: c_ushort,
    ws_col: c_ushort,
    ws_xpixel: c_ushort,
    ws_ypixel: c_ushort,
}

fn get_winsize(fd: c_int) -> io::Result<Size> {
    let mut winsz: Winsize = unsafe { zeroed() };

    // `TIOCGWINSZ.into()` is a workaround to a bug in the libc crate:
    //  https://github.com/rust-lang/libc/pull/704
    let res = unsafe { ioctl(fd, TIOCGWINSZ.into(), &mut winsz) };

    if res == -1 {
        Err(io::Error::last_os_error())
    } else {
        let size = Size{
            lines: winsz.ws_row as usize,
            columns: winsz.ws_col as usize,
        };

        Ok(size)
    }
}

fn nix_to_io(e: nix::Error) -> io::Error {
    io::Error::from_raw_os_error(e as i32)
}

fn ti_to_io(e: terminfo::Error) -> io::Error {
    match e {
        terminfo::Error::Io(e) => e,
        terminfo::Error::NotFound => io::Error::new(
            io::ErrorKind::NotFound, "terminfo entry not found"),
        terminfo::Error::Parse => io::Error::new(
            io::ErrorKind::Other, "failed to parse terminfo entry"),
        terminfo::Error::Expand(_) => io::Error::new(
            io::ErrorKind::Other, "failed to expand terminfo entry"),
    }
}

fn to_timeval(d: Duration) -> TimeVal {
    const MAX_SECS: i64 = i64::max_value() / 1_000;

    let secs = match d.as_secs() {
        n if n > MAX_SECS as u64 => MAX_SECS,
        n => n as i64,
    };

    let millis = d.subsec_millis() as i64;

    TimeVal::milliseconds(secs * 1_000 + millis)
}

fn peek_event(buf: &[u8], sequences: &SeqMap)
        -> io::Result<Option<(Event, usize)>> {
    let (res, n) = {
        let s = utf8_prefix(buf)?;

        if s.is_empty() {
            return Ok(None);
        }

        let mut last_match = None;

        for pfx in prefixes(s) {
            match sequences.find(pfx) {
                FindResult::NotFound => break,
                FindResult::Found(value) => {
                    last_match = Some((pfx, *value));
                    break;
                }
                FindResult::Incomplete => (),
                FindResult::Undecided(value) => {
                    last_match = Some((pfx, *value));
                }
            }
        }

        let res = last_match.and_then(|(seq, value)| {
            match value {
                SeqData::Key(key) => Some((Event::Key(key), seq.len())),
                SeqData::XTermMouse => {
                    if let Some((data, len)) = parse_mouse_data(&buf[seq.len()..]) {
                        Some((Event::Mouse(data), seq.len() + len))
                    } else {
                        // Input sequence was incomplete
                        None
                    }
                }
            }
        });

        if let Some(res) = res {
            res
        } else {
            let ch = s.chars().next().unwrap();
            (Event::Key(ch.into()), ch.len_utf8())
        }
    };

    Ok(Some((res, n)))
}

fn parse_mouse_data(mut buf: &[u8]) -> Option<(MouseEvent, usize)> {
    let orig_len = buf.len();

    let (mut input, end) = parse_integer(&mut buf)?;

    if end != b';' {
        return None;
    }

    let (column, end) = parse_integer(&mut buf)?;

    if end != b';' {
        return None;
    }

    let (line, end) = parse_integer(&mut buf)?;

    let is_pressed = match end {
        b'M' => true,
        b'm' => false,
        _ => return None
    };

    let mut mods = ModifierState::empty();

    if (input & XTERM_SHIFT_MASK) != 0 {
        mods |= ModifierState::SHIFT;
    }
    if (input & XTERM_META_MASK) != 0 {
        mods |= ModifierState::ALT;
    }
    if (input & XTERM_CTRL_MASK) != 0 {
        mods |= ModifierState::CTRL;
    }

    input &= !XTERM_MODIFIER_MASK;

    let input = match input {
        0 ..= 3 => mouse_button_event(input, is_pressed),
        64 => MouseInput::WheelUp,
        65 => MouseInput::WheelDown,
        _ => MouseInput::Motion,
    };

    let position = Cursor{
        // Parsed line and column begin at 1; we begin at 0
        line: (line - 1) as usize,
        column: (column - 1) as usize,
    };

    Some((MouseEvent{
        position,
        input,
        modifiers: mods,
    }, orig_len - buf.len()))
}

fn parse_integer(buf: &mut &[u8]) -> Option<(u32, u8)> {
    let mut n = 0u32;
    let mut iter = buf.iter();

    while let Some(&b) = iter.next() {
        match b {
            b'0' ..= b'9' => {
                n = n.checked_mul(10)?
                    .checked_add((b - b'0') as u32)?;
            }
            _ => {
                *buf = iter.as_slice();
                return Some((n, b));
            }
        }
    }

    None
}

fn mouse_button_event(input: u32, is_pressed: bool) -> MouseInput {
    let button = match input {
        0 => MouseButton::Left,
        1 => MouseButton::Middle,
        2 => MouseButton::Right,
        _ => MouseButton::Other(input)
    };

    if is_pressed {
        MouseInput::ButtonPressed(button)
    } else {
        MouseInput::ButtonReleased(button)
    }
}

fn utf8_prefix(buf: &[u8]) -> io::Result<&str> {
    match from_utf8(buf) {
        Ok(s) => Ok(s),
        Err(e) => {
            if e.valid_up_to() != 0 {
                from_utf8(&buf[..e.valid_up_to()])
                    .map_err(|_| unreachable!())
            } else if e.error_len().is_some() {
                Err(io::Error::new(io::ErrorKind::Other,
                    "read invalid utf-8 data from terminal"))
            } else {
                Ok("")
            }
        }
    }
}

static LAST_SIGNAL: AtomicUsize = AtomicUsize::new(0);

extern "C" fn handle_signal(signum: c_int) {
    LAST_SIGNAL.store(signum as usize, Ordering::Relaxed);
}

fn conv_signal(sig: c_int) -> Option<Signal> {
    match NixSignal::try_from(sig).ok() {
        Some(NixSignal::SIGCONT)  => Some(Signal::Continue),
        Some(NixSignal::SIGINT)   => Some(Signal::Interrupt),
        Some(NixSignal::SIGQUIT)  => Some(Signal::Quit),
        Some(NixSignal::SIGTSTP)  => Some(Signal::Suspend),
        Some(NixSignal::SIGWINCH) => Some(Signal::Resize),
        _ => None
    }
}

fn get_signal() -> Option<Signal> {
    conv_signal(LAST_SIGNAL.load(Ordering::Relaxed) as c_int)
}

fn take_signal() -> Option<Signal> {
    conv_signal(LAST_SIGNAL.swap(0, Ordering::Relaxed) as c_int)
}

fn ascii_str(s: &[u8]) -> Option<&str> {
    use std::str::from_utf8_unchecked;

    if s.is_ascii() {
        Some(unsafe { from_utf8_unchecked(s) })
    } else {
        None
    }
}

fn color_code(color: Color) -> u8 {
    match color {
        Color::Black =>     0,
        Color::Red =>       1,
        Color::Green =>     2,
        Color::Yellow =>    3,
        Color::Blue =>      4,
        Color::Magenta =>   5,
        Color::Cyan =>      6,
        Color::White =>     7,
    }
}

fn not_supported(op: &str) -> io::Error {
    io::Error::new(io::ErrorKind::Other,
        format!("operation not supported: {}", op))
}

#[cfg(target_pointer_width = "64")]
fn to_u32(u: usize) -> u32 {
    if u > u32::max_value() as usize {
        u32::max_value()
    } else {
        u as u32
    }
}

#[cfg(any(target_pointer_width = "16", target_pointer_width = "32"))]
fn to_u32(u: usize) -> u32 {
    u as u32
}
