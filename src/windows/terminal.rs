use std::char;
use std::ffi::OsStr;
use std::io;
use std::mem::{replace, zeroed};
use std::os::windows::ffi::OsStrExt;
use std::ptr;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{LockResult, Mutex, MutexGuard, TryLockResult};
use std::time::Duration;

use winapi::ctypes::c_int;
use winapi::shared::winerror::{
    WAIT_TIMEOUT,
};
use winapi::shared::minwindef::{
    FALSE, TRUE,
    BOOL, DWORD, WORD,
};
use winapi::shared::ntdef::{
    CHAR, SHORT, VOID, WCHAR, HANDLE,
};
use winapi::um::consoleapi::{
    SetConsoleCtrlHandler,
    GetConsoleMode,
    ReadConsoleW,
    ReadConsoleInputW,
    WriteConsoleW,
    SetConsoleMode,
};
use winapi::um::handleapi::{
    CloseHandle,
};
use winapi::um::processenv::{
    GetStdHandle,
};
use winapi::um::synchapi::{
    WaitForSingleObject,
};
use winapi::um::winbase::{
    INFINITE,
    STD_INPUT_HANDLE, STD_OUTPUT_HANDLE, STD_ERROR_HANDLE,
    WAIT_FAILED, WAIT_OBJECT_0,
};
use winapi::um::wincon::{
    self,
    CreateConsoleScreenBuffer,
    WriteConsoleInputW,
    FillConsoleOutputAttribute,
    FillConsoleOutputCharacterA,
    ScrollConsoleScreenBufferW,
    SetConsoleActiveScreenBuffer,
    SetConsoleCursorInfo,
    SetConsoleCursorPosition,
    SetConsoleScreenBufferSize,
    GetConsoleScreenBufferInfo,
    SetConsoleTextAttribute,
    SetConsoleWindowInfo,
    CHAR_INFO, CHAR_INFO_Char, CONSOLE_CURSOR_INFO, CONSOLE_SCREEN_BUFFER_INFO,
    COORD, SMALL_RECT,
    CONSOLE_TEXTMODE_BUFFER,
    INPUT_RECORD,
    CTRL_BREAK_EVENT, CTRL_C_EVENT,
    ENABLE_ECHO_INPUT, ENABLE_LINE_INPUT, ENABLE_MOUSE_INPUT,
    ENABLE_EXTENDED_FLAGS, ENABLE_QUICK_EDIT_MODE, ENABLE_WINDOW_INPUT,
    DISABLE_NEWLINE_AUTO_RETURN,
    ENABLE_VIRTUAL_TERMINAL_INPUT,
    ENABLE_PROCESSED_INPUT,
    ENABLE_PROCESSED_OUTPUT, ENABLE_WRAP_AT_EOL_OUTPUT,
    KEY_EVENT, MOUSE_EVENT, WINDOW_BUFFER_SIZE_EVENT,
};
use winapi::um::winuser;
use winapi::um::winnt::{
    GENERIC_READ, GENERIC_WRITE,
    FILE_SHARE_READ, FILE_SHARE_WRITE,
};

use crate::priv_util::{map_lock_result, map_try_lock_result};
use crate::signal::{Signal, SignalSet};
use crate::terminal::{
    Color, Cursor, CursorMode, Event, Key, PrepareConfig, Size, Style, Theme,
    MouseButton, MouseEvent, MouseInput, ModifierState,
};
use crate::util::unctrl_lower;

pub struct Terminal {
    in_handle: HANDLE,
    default_attrs: WORD,
    old_out_mode: DWORD,
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

unsafe impl Send for Terminal {}
unsafe impl Sync for Terminal {}

struct Reader {
    always_track_motion: bool,
    prev_buttons: DWORD,
}

struct Writer {
    out_handle: HANDLE,
    fg: Option<Color>,
    bg: Option<Color>,
    style: Style,
}

pub struct PrepareState {
    old_in_mode: DWORD,
    clear_handler: bool,
}

impl Terminal {
    fn new(out: DWORD) -> io::Result<Terminal> {
        let in_handle = result_handle(
            unsafe { GetStdHandle(STD_INPUT_HANDLE) })?;
        let out_handle = result_handle(
            unsafe { GetStdHandle(out) })?;

        let default_attrs = unsafe { console_info(out_handle)?.wAttributes };

        let old_out_mode = unsafe { prepare_output(out_handle)? };

        Ok(Terminal{
            in_handle,
            default_attrs,
            old_out_mode,
            reader: Mutex::new(Reader{
                always_track_motion: false,
                prev_buttons: 0,
            }),
            writer: Mutex::new(Writer{
                out_handle,
                fg: None,
                bg: None,
                style: Style::empty(),
            }),
        })
    }

    pub fn stdout() -> io::Result<Terminal> {
        Terminal::new(STD_OUTPUT_HANDLE)
    }

    pub fn stderr() -> io::Result<Terminal> {
        Terminal::new(STD_ERROR_HANDLE)
    }

    pub fn name(&self) -> &str {
        "windows-console"
    }

    pub fn size(&self) -> io::Result<Size> {
        self.lock_writer().size()
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

    pub fn move_to_first_column(&self) -> io::Result<()> {
        self.lock_writer().move_to_first_column()
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

    pub fn enter_screen(&self) -> io::Result<HANDLE> {
        self.lock_writer().enter_screen()
    }

    // This method is unsafe because the validity of `old_handle` cannot be
    // verified. The caller must guarantee that it is the same `HANDLE`
    // previously returned by `enter_screen`.
    pub unsafe fn exit_screen(&self, old_handle: HANDLE) -> io::Result<()> {
        self.lock_writer().exit_screen(old_handle)
    }

    pub fn prepare(&self, config: PrepareConfig) -> io::Result<PrepareState> {
        self.lock_reader().prepare(config)
    }

    pub fn restore(&self, state: PrepareState) -> io::Result<()> {
        self.lock_reader().restore(state)
    }

    pub fn wait_event(&self, timeout: Option<Duration>) -> io::Result<bool> {
        self.lock_reader().wait_event(timeout)
    }

    pub fn read_event(&self, timeout: Option<Duration>) -> io::Result<Option<Event>> {
        self.lock_reader().read_event(timeout)
    }

    pub fn read_raw(&self, buf: &mut [u16],
            timeout: Option<Duration>) -> io::Result<Option<Event>> {
        self.lock_reader().read_raw(buf, timeout)
    }

    pub fn read_raw_event(&self, events: &mut [INPUT_RECORD],
            timeout: Option<Duration>) -> io::Result<Option<Event>> {
        self.lock_reader().read_raw_event(events, timeout)
    }

    pub fn set_cursor_mode(&self, mode: CursorMode) -> io::Result<()> {
        self.lock_writer().set_cursor_mode(mode)
    }

    pub fn clear_attributes(&self) -> io::Result<()> {
        self.lock_writer().clear_attributes()
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

    pub fn set_fg(&self, fg: Option<Color>) -> io::Result<()> {
        self.lock_writer().set_fg(fg)
    }

    pub fn set_bg(&self, bg: Option<Color>) -> io::Result<()> {
        self.lock_writer().set_bg(bg)
    }

    pub fn set_theme(&self, theme: Theme) -> io::Result<()> {
        self.lock_writer().set_theme(theme)
    }

    pub fn write_char(&self, ch: char) -> io::Result<()> {
        self.lock_writer().write_str(ch.encode_utf8(&mut [0; 4]))
    }

    pub fn write_str(&self, s: &str) -> io::Result<()> {
        self.lock_writer().write_str(s)
    }

    pub fn write_styled(&self,
            fg: Option<Color>, bg: Option<Color>, style: Style, text: &str)
            -> io::Result<()> {
        self.lock_writer().write_styled(fg, bg, style, text)
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
        let r = self.set_cursor_mode(CursorMode::Normal);
        let r2 = r.and_then(|_| {
            let lock = self.lock_writer();
            unsafe { set_console_mode(lock.writer.out_handle, self.old_out_mode)?; }
            Ok(())
        });

        if let Err(e) = r2 {
            eprintln!("failed to restore terminal: {}", e);
        }
    }
}

impl<'a> TerminalReadGuard<'a> {
    fn new(term: &'a Terminal, reader: MutexGuard<'a, Reader>) -> TerminalReadGuard<'a> {
        TerminalReadGuard{term, reader}
    }

    pub fn prepare(&mut self, config: PrepareConfig) -> io::Result<PrepareState> {
        // The write lock is acquired here for consistency, though it is not used
        // in the Windows implementation. This is done to ensure that a user will
        // not write and test Windows code that then causes a deadlock on Unix.
        let mut writer = self.term.lock_writer();
        self.prepare_with_lock(&mut writer, config)
    }

    pub fn prepare_with_lock(&mut self, _writer: &mut TerminalWriteGuard,
            config: PrepareConfig) -> io::Result<PrepareState> {
        let old_in_mode = unsafe { console_mode(self.term.in_handle)? };

        let mut state = PrepareState{
            old_in_mode,
            clear_handler: false,
        };

        let mut in_mode = old_in_mode;

        // Necessary to modify certain flags
        in_mode |= ENABLE_EXTENDED_FLAGS;

        // Disable echoing input to console
        in_mode &= !ENABLE_ECHO_INPUT;
        // Disable waiting for newline before input can be read
        in_mode &= !ENABLE_LINE_INPUT;

        // Enable or disable processing Ctrl-C as interrupt
        if config.block_signals {
            in_mode &= !ENABLE_PROCESSED_INPUT;
        } else {
            in_mode |= ENABLE_PROCESSED_INPUT;
        }

        // Enable or disable mouse events
        if config.enable_mouse {
            self.reader.always_track_motion = config.always_track_motion;
            in_mode |= ENABLE_MOUSE_INPUT;
        } else {
            in_mode &= !ENABLE_MOUSE_INPUT;
        }

        // Disable text editing using mouse
        in_mode &= !ENABLE_QUICK_EDIT_MODE;

        // Enable window size events
        in_mode |= ENABLE_WINDOW_INPUT;

        // Disable escape sequences in input
        in_mode &= !ENABLE_VIRTUAL_TERMINAL_INPUT;

        unsafe {
            set_console_mode(self.term.in_handle, in_mode)?;

            if config.report_signals.intersects(Signal::Break | Signal::Interrupt) {
                catch_signals(config.report_signals);
                result_bool(SetConsoleCtrlHandler(Some(ctrl_handler), TRUE))?;
                state.clear_handler = true;
            }
        }

        Ok(state)
    }

    pub fn restore(&mut self, state: PrepareState) -> io::Result<()> {
        let mut writer = self.term.lock_writer();
        self.restore_with_lock(&mut writer, state)
    }

    pub fn restore_with_lock(&mut self, _writer: &mut TerminalWriteGuard,
            state: PrepareState) -> io::Result<()> {
        unsafe {
            if state.clear_handler {
                result_bool(SetConsoleCtrlHandler(Some(ctrl_handler), FALSE))?;
            }

            set_console_mode(self.term.in_handle,
                state.old_in_mode | ENABLE_EXTENDED_FLAGS)?;
        }

        Ok(())
    }

    pub fn wait_event(&mut self, timeout: Option<Duration>) -> io::Result<bool> {
        if get_signal().is_some() {
            return Ok(true);
        }

        let res = unsafe { WaitForSingleObject(
            self.term.in_handle, as_millis(timeout)) };

        match res {
            WAIT_OBJECT_0 => Ok(true),
            WAIT_TIMEOUT => Ok(false),
            WAIT_FAILED | _ => Err(io::Error::last_os_error())
        }
    }

    pub fn read_event(&mut self, timeout: Option<Duration>) -> io::Result<Option<Event>> {
        let mut event: [INPUT_RECORD; 1] = unsafe { zeroed() };

        let n = match self.read_raw_event(&mut event, timeout)? {
            Some(Event::Raw(n)) => n,
            r => return Ok(r)
        };

        if n == 0 {
            Ok(None)
        } else {
            let event = event[0];

            if let Some(key) = key_press_event(&event) {
                Ok(Some(Event::Key(key)))
            } else if let Some(mouse) = self.mouse_event(&event) {
                Ok(Some(Event::Mouse(mouse)))
            } else if let Some(size) = size_event(&event) {
                Ok(Some(Event::Resize(size)))
            } else {
                Ok(Some(Event::NoEvent))
            }
        }
    }

    pub fn read_raw(&mut self, buf: &mut [u16], timeout: Option<Duration>)
            -> io::Result<Option<Event>> {
        if !self.wait_event(timeout)? {
            return Ok(None);
        }

        if let Some(sig) = take_signal() {
            return Ok(Some(Event::Signal(sig)));
        }

        unsafe {
            let len = to_dword(buf.len());
            let mut n_read = 0;

            result_bool(ReadConsoleW(
                self.term.in_handle,
                buf.as_ptr() as *mut VOID,
                len,
                &mut n_read,
                ptr::null_mut()))?;

            if n_read == 0 {
                Ok(None)
            } else {
                Ok(Some(Event::Raw(n_read as usize)))
            }
        }
    }

    pub fn read_raw_event(&mut self, events: &mut [INPUT_RECORD],
            timeout: Option<Duration>) -> io::Result<Option<Event>> {
        if !self.wait_event(timeout)? {
            return Ok(None);
        }

        if let Some(sig) = take_signal() {
            return Ok(Some(Event::Signal(sig)));
        }

        let len = to_dword(events.len());
        let mut n = 0;

        result_bool(unsafe { ReadConsoleInputW(
            self.term.in_handle,
            events.as_mut_ptr(),
            len,
            &mut n) })?;

        Ok(Some(Event::Raw(n as usize)))
    }

    fn mouse_event(&mut self, event: &INPUT_RECORD) -> Option<MouseEvent> {
        if event.EventType == MOUSE_EVENT {
            let mouse = unsafe { event.Event.MouseEvent() };

            let input = if mouse.dwEventFlags & wincon::MOUSE_WHEELED != 0 {
                // The high word of `dwButtonState` indicates wheel direction
                let direction = (mouse.dwButtonState >> 16) as i16;

                if direction > 0 {
                    MouseInput::WheelUp
                } else {
                    MouseInput::WheelDown
                }
            } else {
                let prev_buttons = self.reader.prev_buttons;
                let now_buttons = mouse.dwButtonState;

                self.reader.prev_buttons = mouse.dwButtonState;

                if prev_buttons == now_buttons {
                    if now_buttons == 0 && !self.reader.always_track_motion {
                        return None;
                    }

                    MouseInput::Motion
                } else {
                    button_changed(prev_buttons, now_buttons)?
                }
            };

            let position = coord_to_cursor(mouse.dwMousePosition);

            let mut mods = ModifierState::empty();

            if has_alt(mouse.dwControlKeyState) {
                mods |= ModifierState::ALT;
            }
            if has_ctrl(mouse.dwControlKeyState) {
                mods |= ModifierState::CTRL;
            }
            if has_shift(mouse.dwControlKeyState) {
                mods |= ModifierState::SHIFT;
            }

            Some(MouseEvent{
                position,
                input,
                modifiers: mods,
            })
        } else {
            None
        }
    }
}

impl<'a> TerminalWriteGuard<'a> {
    fn new(term: &'a Terminal, writer: MutexGuard<'a, Writer>) -> TerminalWriteGuard<'a> {
        TerminalWriteGuard{term, writer: writer}
    }

    fn enter_screen(&mut self) -> io::Result<HANDLE> {
        let size = self.size()?;

        let handle = result_handle(unsafe { CreateConsoleScreenBuffer(
            GENERIC_READ | GENERIC_WRITE,
            FILE_SHARE_READ | FILE_SHARE_WRITE,
            ptr::null(),
            CONSOLE_TEXTMODE_BUFFER,
            ptr::null_mut()) })?;

        if let Err(e) = unsafe { setup_screen(handle, size) } {
            // If setup fails, close the screen buffer handle,
            // but preserve the original error
            let _ = unsafe { close_handle(handle) };
            return Err(e);
        }

        let old_handle = self.swap_out_handle(handle);

        let mut out_mode = unsafe { console_mode(handle)? };

        // Disable wrapping when cursor passes last column
        out_mode &= !(ENABLE_WRAP_AT_EOL_OUTPUT | DISABLE_NEWLINE_AUTO_RETURN);

        unsafe { set_console_mode(handle, out_mode)?; }

        Ok(old_handle)
    }

    unsafe fn exit_screen(&mut self, old_handle: HANDLE) -> io::Result<()> {
        result_bool(SetConsoleActiveScreenBuffer(old_handle))?;

        let handle = self.swap_out_handle(old_handle);

        close_handle(handle)
    }

    pub fn size(&self) -> io::Result<Size> {
        unsafe { console_size(self.writer.out_handle) }
    }

    pub fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }

    pub fn clear_screen(&mut self) -> io::Result<()> {
        let mut info = self.get_info()?;

        let win_height = (info.srWindow.Bottom - info.srWindow.Top) + 1;

        if win_height == info.dwSize.Y {
            // Window and screen buffer are the same size. Just erase everything.
            self.clear_area(
                COORD{X: 0, Y: 0},
                info.dwSize.X as DWORD * info.dwSize.Y as DWORD)?;
        } else {
            // Distance we can move down
            let max = info.dwSize.Y - (info.srWindow.Bottom + 1);
            // Distance we want to move down
            let dist = (info.dwCursorPosition.Y + 1) - info.srWindow.Top;

            let down = dist.min(max);

            // If there's room to move the window down, do it
            if down > 0 {
                info.srWindow.Top += down as SHORT;
                info.srWindow.Bottom += down as SHORT;

                result_bool(unsafe { SetConsoleWindowInfo(
                    self.writer.out_handle,
                    TRUE,
                    &info.srWindow) })?;
            }

            let clear = info.srWindow.Bottom - info.dwCursorPosition.Y;

            // If we need to move some text, do that, too
            if clear < win_height {
                let dist = (win_height - clear) as SHORT;

                let src = SMALL_RECT{
                    Top: dist,
                    Bottom: info.dwCursorPosition.Y,
                    Left: 0,
                    Right: info.dwSize.X,
                };

                let dest = COORD{
                    X: 0,
                    Y: 0,
                };

                let fill = CHAR_INFO{
                    Char: unicode_char(b' ' as WCHAR),
                    Attributes: 0,
                };

                result_bool(unsafe { ScrollConsoleScreenBufferW(
                    self.writer.out_handle,
                    &src,
                    ptr::null(),
                    dest,
                    &fill) })?;
            }
        }

        // Finally, move the cursor to the window origin
        self.move_abs(COORD{
            X: info.srWindow.Left,
            Y: info.srWindow.Top,
        })
    }

    pub fn clear_to_line_end(&mut self) -> io::Result<()> {
        let info = self.get_info()?;

        let start = info.dwCursorPosition;
        let size = info.dwSize;

        self.clear_area(start, (size.X - start.X) as DWORD)
    }

    pub fn clear_to_screen_end(&mut self) -> io::Result<()> {
        let info = self.get_info()?;

        let start = info.dwCursorPosition;
        let size = info.dwSize;

        let lines = (size.Y - start.Y) as DWORD;
        let columns = (size.X - start.X) as DWORD;

        let n = lines * size.X as DWORD + columns;

        self.clear_area(start, n)
    }

    pub fn move_cursor(&mut self, pos: Cursor) -> io::Result<()> {
        self.move_abs(cursor_to_coord(pos))
    }

    pub fn move_to_first_column(&mut self) -> io::Result<()> {
        let info = self.get_info()?;
        self.move_abs(COORD{X: 0, Y: info.dwCursorPosition.Y})
    }

    pub fn move_up(&mut self, n: usize) -> io::Result<()> {
        self.move_rel(COORD{X: 0, Y: to_short_neg(n)})
    }

    pub fn move_down(&mut self, n: usize) -> io::Result<()> {
        self.move_rel(COORD{X: 0, Y: to_short(n)})
    }

    pub fn move_left(&mut self, n: usize) -> io::Result<()> {
        self.move_rel(COORD{X: to_short_neg(n), Y: 0})
    }

    pub fn move_right(&mut self, n: usize) -> io::Result<()> {
        self.move_rel(COORD{X: to_short(n), Y: 0})
    }

    pub fn set_cursor_mode(&mut self, mode: CursorMode) -> io::Result<()> {
        let (size, vis) = match mode {
            CursorMode::Normal => (25, TRUE),
            CursorMode::Invisible => (1, FALSE),
            CursorMode::Overwrite => (100, TRUE),
        };

        let info = CONSOLE_CURSOR_INFO {
            dwSize: size,
            bVisible: vis,
        };

        result_bool(unsafe { SetConsoleCursorInfo(self.writer.out_handle, &info) })
    }

    pub fn clear_attributes(&mut self) -> io::Result<()> {
        self.set_attributes(None, None, Style::empty())
    }

    pub fn add_style(&mut self, style: Style) -> io::Result<()> {
        let add = style - self.writer.style;

        if !add.is_empty() {
            self.writer.style |= add;
            self.update_attrs()?;
        }

        Ok(())
    }

    pub fn remove_style(&mut self, style: Style) -> io::Result<()> {
        let remove = style & self.writer.style;

        if !remove.is_empty() {
            self.writer.style -= remove;
            self.update_attrs()?;
        }

        Ok(())
    }

    pub fn set_style(&mut self, style: Style) -> io::Result<()> {
        if self.writer.style != style {
            self.writer.style = style;
            self.update_attrs()?;
        }
        Ok(())
    }

    pub fn set_fg(&mut self, fg: Option<Color>) -> io::Result<()> {
        if self.writer.fg != fg {
            self.writer.fg = fg;
            self.update_attrs()?;
        }

        Ok(())
    }

    pub fn set_bg(&mut self, bg: Option<Color>) -> io::Result<()> {
        if self.writer.bg != bg {
            self.writer.bg = bg;
            self.update_attrs()?;
        }
        Ok(())
    }

    pub fn set_theme(&mut self, theme: Theme) -> io::Result<()> {
        self.set_attributes(theme.fg, theme.bg, theme.style)
    }

    // Clears any previous attributes
    pub fn set_attributes(&mut self,
            fg: Option<Color>, bg: Option<Color>, style: Style) -> io::Result<()> {
        if self.writer.fg != fg || self.writer.bg != bg || self.writer.style != style {
            self.writer.fg = fg;
            self.writer.bg = bg;
            self.writer.style = style;
            self.update_attrs()?;
        }

        Ok(())
    }

    fn update_attrs(&mut self) -> io::Result<()> {
        let mut attrs = self.term.default_attrs;

        if let Some(fg) = self.writer.fg {
            attrs &= !fg_code(Color::White);
            attrs |= fg_code(fg);
        }

        if let Some(bg) = self.writer.bg {
            attrs &= !bg_code(Color::White);
            attrs |= bg_code(bg);
        }

        attrs |= style_code(self.writer.style);

        if self.writer.style.contains(Style::REVERSE) {
            attrs = swap_colors(attrs);
        }

        self.set_attrs(attrs)
    }

    pub fn write_char(&mut self, ch: char) -> io::Result<()> {
        let mut buf = [0; 4];
        self.write_str(ch.encode_utf8(&mut buf))
    }

    pub fn write_str(&mut self, s: &str) -> io::Result<()> {
        let buf = OsStr::new(s).encode_wide().collect::<Vec<_>>();
        let mut n = 0;

        while buf.len() > n {
            let mut n_dw = 0;
            let len = to_dword(buf.len() - n);

            result_bool(unsafe { WriteConsoleW(
                self.writer.out_handle,
                buf[n..].as_ptr() as *const VOID,
                len,
                &mut n_dw,
                ptr::null_mut()) })?;

            n += n_dw as usize;
        }

        Ok(())
    }

    pub fn write_styled(&mut self,
            fg: Option<Color>, bg: Option<Color>, style: Style, text: &str)
            -> io::Result<()> {
        self.set_attributes(fg, bg, style)?;
        self.write_str(text)?;
        self.clear_attributes()
    }

    fn clear_area(&mut self, start: COORD, n: DWORD) -> io::Result<()> {
        let mut n_chars = 0;

        result_bool(unsafe { FillConsoleOutputAttribute(
            self.writer.out_handle,
            self.term.default_attrs,
            n,
            start,
            &mut n_chars) })?;

        result_bool(unsafe { FillConsoleOutputCharacterA(
            self.writer.out_handle,
            b' ' as CHAR,
            n,
            start,
            &mut n_chars) })?;

        Ok(())
    }

    fn move_abs(&mut self, pos: COORD) -> io::Result<()> {
        result_bool(unsafe { SetConsoleCursorPosition(
            self.writer.out_handle, pos) })
    }

    fn move_rel(&mut self, off: COORD) -> io::Result<()> {
        let info = self.get_info()?;

        let size = info.dwSize;
        let cursor = info.dwCursorPosition;

        let dest = COORD{
            X: cursor.X.saturating_add(off.X).min(size.X - 1),
            Y: cursor.Y.saturating_add(off.Y).min(size.Y - 1),
        };

        self.move_abs(dest)
    }

    fn set_attrs(&mut self, attrs: WORD) -> io::Result<()> {
        result_bool(unsafe { SetConsoleTextAttribute(
            self.writer.out_handle, attrs) })
    }

    fn get_info(&self) -> io::Result<CONSOLE_SCREEN_BUFFER_INFO> {
        unsafe { console_info(self.writer.out_handle) }
    }

    fn swap_out_handle(&mut self, handle: HANDLE) -> HANDLE {
        replace(&mut self.writer.out_handle, handle)
    }
}

// NOTE: Drop is not implemented for TerminalWriteGuard
// because `flush` on Windows is a no-op.

fn as_millis(timeout: Option<Duration>) -> DWORD {
    match timeout {
        Some(t) => {
            let s = (t.as_secs() * 1_000) as DWORD;
            let ms = (t.subsec_nanos() / 1_000_000) as DWORD;

            s + ms
        }
        None => INFINITE,
    }
}

fn fg_code(color: Color) -> WORD {
    (match color {
        Color::Black => 0,
        Color::Blue => wincon::FOREGROUND_BLUE,
        Color::Cyan => wincon::FOREGROUND_BLUE | wincon::FOREGROUND_GREEN,
        Color::Green => wincon::FOREGROUND_GREEN,
        Color::Magenta => wincon::FOREGROUND_BLUE | wincon::FOREGROUND_RED,
        Color::Red => wincon::FOREGROUND_RED,
        Color::White => wincon::FOREGROUND_RED | wincon::FOREGROUND_GREEN | wincon::FOREGROUND_BLUE,
        Color::Yellow => wincon::FOREGROUND_RED | wincon::FOREGROUND_GREEN,
    }) as WORD
}

fn bg_code(color: Color) -> WORD {
    (match color {
        Color::Black => 0,
        Color::Blue => wincon::BACKGROUND_BLUE,
        Color::Cyan => wincon::BACKGROUND_BLUE | wincon::BACKGROUND_GREEN,
        Color::Green => wincon::BACKGROUND_GREEN,
        Color::Magenta => wincon::BACKGROUND_BLUE | wincon::BACKGROUND_RED,
        Color::Red => wincon::BACKGROUND_RED,
        Color::White => wincon::BACKGROUND_RED | wincon::BACKGROUND_GREEN | wincon::BACKGROUND_BLUE,
        Color::Yellow => wincon::BACKGROUND_RED | wincon::BACKGROUND_GREEN,
    }) as WORD
}

fn style_code(style: Style) -> WORD {
    let mut code = 0;

    if style.contains(Style::BOLD) {
        // Closest available approximation for bold text
        code |= wincon::FOREGROUND_INTENSITY as WORD;
    }

    code
}

fn swap_colors(code: WORD) -> WORD {
    let fg_mask = fg_code(Color::White);
    let bg_mask = bg_code(Color::White);

    let fg_shift = fg_mask.trailing_zeros();
    let bg_shift = bg_mask.trailing_zeros();
    let shift = bg_shift - fg_shift;

    let fg = code & fg_mask;
    let bg = code & bg_mask;

    let swapped_fg = fg << shift;
    let swapped_bg = bg >> shift;

    (code & !(fg_mask | bg_mask)) | swapped_fg | swapped_bg
}

unsafe fn close_handle(handle: HANDLE) -> io::Result<()> {
    result_bool(CloseHandle(handle))
}

unsafe fn console_info(handle: HANDLE) -> io::Result<CONSOLE_SCREEN_BUFFER_INFO> {
    let mut info = zeroed();

    result_bool(GetConsoleScreenBufferInfo(handle, &mut info))?;

    Ok(info)
}

unsafe fn console_mode(handle: HANDLE) -> io::Result<DWORD> {
    let mut mode = 0;

    result_bool(GetConsoleMode(handle, &mut mode))?;

    Ok(mode)
}

unsafe fn console_size(handle: HANDLE) -> io::Result<Size> {
    let info = console_info(handle)?;

    Ok(coord_to_size(info.dwSize))
}

unsafe fn set_console_mode(handle: HANDLE, mode: DWORD) -> io::Result<()> {
    result_bool(SetConsoleMode(handle, mode))
}

// Perform remaining screen buffer setup
unsafe fn setup_screen(handle: HANDLE, size: Size) -> io::Result<()> {
    result_bool(SetConsoleScreenBufferSize(handle, size_to_coord(size)))?;
    result_bool(SetConsoleActiveScreenBuffer(handle))
}

unsafe fn prepare_output(handle: HANDLE) -> io::Result<DWORD> {
    let old_out_mode = console_mode(handle)?;

    let mut out_mode = old_out_mode;

    // Enable interpreting escape sequences in output
    out_mode |= ENABLE_PROCESSED_OUTPUT;

    // Enable wrapping when cursor passes last column
    out_mode |= ENABLE_WRAP_AT_EOL_OUTPUT;

    set_console_mode(handle, out_mode)?;

    Ok(old_out_mode)
}

fn button_changed(prev_buttons: DWORD, now_buttons: DWORD) -> Option<MouseInput> {
    use std::mem::size_of;

    let n_bits = size_of::<DWORD>() * 8;

    for i in 0..n_bits {
        let bit = 1 << i;

        let changed = (prev_buttons ^ now_buttons) & bit != 0;

        if changed {
            let button = bit_to_button(bit);

            let input = if prev_buttons & bit == 0 {
                MouseInput::ButtonPressed(button)
            } else {
                MouseInput::ButtonReleased(button)
            };

            return Some(input);
        }
    }

    None
}

fn bit_to_button(mut bit: DWORD) -> MouseButton {
    assert!(bit != 0);

    match bit {
        wincon::FROM_LEFT_1ST_BUTTON_PRESSED => MouseButton::Left,
        wincon::RIGHTMOST_BUTTON_PRESSED => MouseButton::Right,
        wincon::FROM_LEFT_2ND_BUTTON_PRESSED => MouseButton::Middle,
        _ => {
            bit >>= 3;
            let mut n = 3;

            while bit != 1 {
                bit >>= 1;
                n += 1;
            }

            MouseButton::Other(n)
        }
    }
}

fn coord_to_cursor(pos: COORD) -> Cursor {
    Cursor{
        line: pos.Y as usize,
        column: pos.X as usize,
    }
}

fn coord_to_size(size: COORD) -> Size {
    Size{
        lines: size.Y as usize,
        columns: size.X as usize,
    }
}

fn cursor_to_coord(pos: Cursor) -> COORD {
    COORD{
        Y: to_short(pos.line),
        X: to_short(pos.column),
    }
}

fn size_to_coord(size: Size) -> COORD {
    COORD{
        Y: to_short(size.lines),
        X: to_short(size.columns),
    }
}

fn has_alt(state: DWORD) -> bool {
    state & (wincon::LEFT_ALT_PRESSED | wincon::RIGHT_ALT_PRESSED) != 0
}

fn has_ctrl(state: DWORD) -> bool {
    state & (wincon::LEFT_CTRL_PRESSED | wincon::RIGHT_CTRL_PRESSED) != 0
}

fn has_shift(state: DWORD) -> bool {
    state & wincon::SHIFT_PRESSED != 0
}

fn to_dword(n: usize) -> DWORD {
    if n > DWORD::max_value() as usize {
        DWORD::max_value()
    } else {
        n as DWORD
    }
}

fn to_short(n: usize) -> SHORT {
    if n > SHORT::max_value() as usize {
        SHORT::max_value()
    } else {
        n as SHORT
    }
}

fn to_short_neg(n: usize) -> SHORT {
    let n = if n > isize::max_value() as usize {
        isize::min_value()
    } else {
        -(n as isize)
    };

    if n < SHORT::min_value() as isize {
        SHORT::min_value()
    } else {
        n as SHORT
    }
}

fn key_press_event(event: &INPUT_RECORD) -> Option<Key> {
    if event.EventType == KEY_EVENT {
        let key = unsafe { event.Event.KeyEvent() };

        if key.bKeyDown == FALSE {
            return None;
        }

        let key = match key.wVirtualKeyCode as c_int {
            winuser::VK_BACK => Key::Backspace,
            winuser::VK_RETURN => Key::Enter,
            winuser::VK_ESCAPE => Key::Escape,
            winuser::VK_TAB => Key::Tab,
            winuser::VK_UP => Key::Up,
            winuser::VK_DOWN => Key::Down,
            winuser::VK_LEFT => Key::Left,
            winuser::VK_RIGHT => Key::Right,
            winuser::VK_DELETE => Key::Delete,
            winuser::VK_INSERT => Key::Insert,
            winuser::VK_HOME => Key::Home,
            winuser::VK_END => Key::End,
            winuser::VK_PRIOR => Key::PageUp,
            winuser::VK_NEXT => Key::PageDown,
            winuser::VK_F1 => Key::F(1),
            winuser::VK_F2 => Key::F(2),
            winuser::VK_F3 => Key::F(3),
            winuser::VK_F4 => Key::F(4),
            winuser::VK_F5 => Key::F(5),
            winuser::VK_F6 => Key::F(6),
            winuser::VK_F7 => Key::F(7),
            winuser::VK_F8 => Key::F(8),
            winuser::VK_F9 => Key::F(9),
            winuser::VK_F10 => Key::F(10),
            winuser::VK_F11 => Key::F(11),
            winuser::VK_F12 => Key::F(12),
            _ => {
                if has_alt(key.dwControlKeyState) {
                    return None;
                }

                let is_ctrl = has_ctrl(key.dwControlKeyState);

                let u_char = unsafe { *key.uChar.UnicodeChar() };

                if u_char != 0 {
                    match char::from_u32(u_char as u32) {
                        Some(ch) if is_ctrl => Key::Ctrl(unctrl_lower(ch)),
                        Some(ch) => ch.into(),
                        None => return None
                    }
                } else {
                    return None;
                }
            }
        };

        Some(key)
    } else {
        None
    }
}

pub fn size_event(event: &INPUT_RECORD) -> Option<Size> {
    if event.EventType == WINDOW_BUFFER_SIZE_EVENT {
        let size = unsafe { event.Event.WindowBufferSizeEvent() };

        Some(Size{
            lines: size.dwSize.Y as usize,
            columns: size.dwSize.X as usize,
        })
    } else {
        None
    }
}

fn unicode_char(wch: WCHAR) -> CHAR_INFO_Char {
    let mut ch: CHAR_INFO_Char = unsafe { zeroed() };

    unsafe { *ch.UnicodeChar_mut() = wch; }

    ch
}

fn result_bool(b: BOOL) -> io::Result<()> {
    if b == FALSE {
        Err(io::Error::last_os_error())
    } else {
        Ok(())
    }
}

fn result_handle(ptr: HANDLE) -> io::Result<HANDLE> {
    if ptr.is_null() {
        Err(io::Error::last_os_error())
    } else {
        Ok(ptr)
    }
}

static CATCH_SIGNALS: AtomicUsize = AtomicUsize::new(0);

// `CTRL_C_EVENT` has a value of 0, so we cannot use 0 as a default value.
// Instead, !0 indicates no signal has been received.
static LAST_SIGNAL: AtomicUsize = AtomicUsize::new(!0);

fn catch_signals(set: SignalSet) {
    let mut sigs = 0;

    if set.contains(Signal::Break) {
        sigs |= (1 << CTRL_BREAK_EVENT) as usize;
    }
    if set.contains(Signal::Interrupt) {
        sigs |= (1 << CTRL_C_EVENT) as usize;
    }

    CATCH_SIGNALS.store(sigs, Ordering::Relaxed);
}

unsafe extern "system" fn ctrl_handler(ctrl_type: DWORD) -> BOOL {
    match ctrl_type {
        CTRL_BREAK_EVENT | CTRL_C_EVENT => {
            let catch = CATCH_SIGNALS.load(Ordering::Relaxed);

            if catch & (1 << ctrl_type) as usize == 0 {
                return FALSE;
            }

            LAST_SIGNAL.store(ctrl_type as usize, Ordering::Relaxed);

            if let Ok(handle) = result_handle(
                    GetStdHandle(STD_INPUT_HANDLE)) {
                // Wake up the `WaitForSingleObject` call by
                // generating a key up event, which will be ignored.
                let input = INPUT_RECORD{
                    EventType: KEY_EVENT,
                    // KEY_EVENT { bKeyDown: FALSE, ... }
                    Event: zeroed(),
                };

                let mut n = 0;

                // Ignore any errors from this
                let _ = WriteConsoleInputW(
                    handle,
                    &input,
                    1,
                    &mut n);
            }

            TRUE
        }
        _ => FALSE
    }
}

fn conv_signal(sig: DWORD) -> Option<Signal> {
    match sig {
        CTRL_BREAK_EVENT => Some(Signal::Break),
        CTRL_C_EVENT => Some(Signal::Interrupt),
        _ => None
    }
}

fn get_signal() -> Option<Signal> {
    conv_signal(LAST_SIGNAL.load(Ordering::Relaxed) as DWORD)
}

fn take_signal() -> Option<Signal> {
    conv_signal(LAST_SIGNAL.swap(!0, Ordering::Relaxed) as DWORD)
}
