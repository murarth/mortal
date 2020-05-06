use std::convert::TryFrom;
use std::io;
use std::mem::{replace, zeroed};
use std::path::Path;
use std::str::from_utf8;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{LockResult, Mutex, MutexGuard, TryLockResult};
use std::time::Duration;

use smallstr::SmallString;

use crate::priv_util::{map_lock_result, map_try_lock_result};
use crate::sequence::{FindResult, SequenceMap};
use crate::signal::{Signal, SignalSet};
use crate::terminal::{
    Color, Cursor, CursorMode, Event, Key, ModifierState, MouseButton, MouseEvent, MouseInput,
    PrepareConfig, Size, Style, Theme,
};
use crate::util::prefixes;

type SeqMap = SequenceMap<SmallString<[u8; 8]>, SeqData>;

#[derive(Copy, Clone)]
enum SeqData {
    XTermMouse,
    Key(Key),
}

pub struct Terminal {
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
    out_buffer: Vec<u8>,
    fg: Option<Color>,
    bg: Option<Color>,
    cur_style: Style,
}

pub struct PrepareState {
    restore_keypad: bool,
    restore_mouse: bool,
    prev_resume: Option<Resume>,
}

#[derive(Copy, Clone, Debug)]
struct Resume {
    config: PrepareConfig,
}

#[repr(C)]
struct Winsize {
    ws_row: usize,
    ws_col: usize,
    ws_xpixel: usize,
    ws_ypixel: usize,
}

impl Terminal {

    fn new() -> io::Result<Terminal> {
        todo!()
    }

    pub fn stdout() -> io::Result<Terminal> {
        todo!()
    }

    pub fn stderr() -> io::Result<Terminal> {
        todo!()
    }

    pub fn name(&self) -> &str {
        todo!()
    }

    pub fn size(&self) -> io::Result<Size> {
        todo!()
    }

    pub fn wait_event(&self, timeout: Option<Duration>) -> io::Result<bool> {
        todo!()
    }

    pub fn read_event(&self, timeout: Option<Duration>) -> io::Result<Option<Event>> {
        todo!()
    }

    pub fn read_raw(&self, buf: &mut [u8], timeout: Option<Duration>) -> io::Result<Option<Event>> {
        todo!()
    }

    pub fn enter_screen(&self) -> io::Result<()> {
        todo!()
    }

    pub fn exit_screen(&self) -> io::Result<()> {
        todo!()
    }

    pub fn prepare(&self, config: PrepareConfig) -> io::Result<PrepareState> {
        todo!()
    }

    pub fn restore(&self, state: PrepareState) -> io::Result<()> {
        todo!()
    }

    pub fn clear_screen(&self) -> io::Result<()> {
        todo!()
    }

    pub fn clear_to_line_end(&self) -> io::Result<()> {
        todo!()
    }

    pub fn clear_to_screen_end(&self) -> io::Result<()> {
        todo!()
    }

    pub fn move_up(&self, n: usize) -> io::Result<()> {
        if n != 0 {
        todo!()
        }
        Ok(())
    }

    pub fn move_down(&self, n: usize) -> io::Result<()> {
        if n != 0 {
        todo!()
        }
        Ok(())
    }

    pub fn move_left(&self, n: usize) -> io::Result<()> {
        if n != 0 {
        todo!()
        }
        Ok(())
    }

    pub fn move_right(&self, n: usize) -> io::Result<()> {
        if n != 0 {
        todo!()
        }
        Ok(())
    }

    pub fn move_to_first_column(&self) -> io::Result<()> {
        todo!()
    }

    pub fn set_cursor_mode(&self, mode: CursorMode) -> io::Result<()> {
        todo!()
    }

    pub fn write_char(&self, ch: char) -> io::Result<()> {
        todo!()
    }

    pub fn write_str(&self, s: &str) -> io::Result<()> {
        todo!()
    }

    pub fn write_styled(&self,
            fg: Option<Color>, bg: Option<Color>, style: Style, text: &str)
            -> io::Result<()> {
        todo!()
    }

    pub fn clear_attributes(&self) -> io::Result<()> {
        todo!()
    }

    pub fn set_fg(&self, fg: Option<Color>) -> io::Result<()> {
        todo!()
    }

    pub fn set_bg(&self, bg: Option<Color>) -> io::Result<()> {
        todo!()
    }

    pub fn add_style(&self, style: Style) -> io::Result<()> {
        todo!()
    }

    pub fn remove_style(&self, style: Style) -> io::Result<()> {
        todo!()
    }

    pub fn set_style(&self, style: Style) -> io::Result<()> {
        todo!()
    }

    pub fn set_theme(&self, theme: Theme) -> io::Result<()> {
        todo!()
    }

    pub fn lock_read(&self) -> LockResult<TerminalReadGuard> {
        todo!()
    }

    pub fn lock_write(&self) -> LockResult<TerminalWriteGuard> {
        todo!()
    }

    pub fn try_lock_read(&self) -> TryLockResult<TerminalReadGuard> {
        todo!()
    }

    pub fn try_lock_write(&self) -> TryLockResult<TerminalWriteGuard> {
        todo!()
    }

    fn lock_reader(&self) -> TerminalReadGuard {
        todo!()
    }

    fn lock_writer(&self) -> TerminalWriteGuard {
        todo!()
    }
}

impl<'a> TerminalReadGuard<'a> {
    fn new(term: &'a Terminal, reader: MutexGuard<'a, Reader>) -> TerminalReadGuard<'a> {
        todo!()
    }

    pub fn prepare(&mut self, config: PrepareConfig) -> io::Result<PrepareState> {
        todo!()
    }

    pub fn prepare_with_lock(&mut self, writer: &mut TerminalWriteGuard,
            config: PrepareConfig) -> io::Result<PrepareState> {
        todo!()
    }

    pub fn restore(&mut self, state: PrepareState) -> io::Result<()> {
        todo!()
    }

    pub fn restore_with_lock(&mut self, writer: &mut TerminalWriteGuard,
            state: PrepareState) -> io::Result<()> {
        todo!()
    }

    pub fn wait_event(&mut self, timeout: Option<Duration>) -> io::Result<bool> {
        todo!()
    }

    pub fn read_event(&mut self, timeout: Option<Duration>) -> io::Result<Option<Event>> {
        todo!()
    }

    pub fn read_raw(&mut self, buf: &mut [u8], timeout: Option<Duration>) -> io::Result<Option<Event>> {
        todo!()
    }

    fn read_into_buffer(&mut self, timeout: Option<Duration>) -> io::Result<Option<Event>> {
        todo!()
    }

    fn read_input(&mut self, buf: &mut [u8], timeout: Option<Duration>) -> io::Result<Option<Event>> {
        todo!()
    }

    fn try_read(&mut self) -> io::Result<Option<Event>> {
        todo!()
    }

    fn handle_signal(&mut self, sig: Signal) -> io::Result<Option<Event>> {
        todo!()
    }

    fn resume(&mut self) -> io::Result<()> {
        todo!()
    }
}

impl<'a> TerminalWriteGuard<'a> {
    fn new(term: &'a Terminal, writer: MutexGuard<'a, Writer>) -> TerminalWriteGuard<'a> {
        TerminalWriteGuard{term, writer}
    }

    pub fn size(&self) -> io::Result<Size> {
        todo!()
    }

    fn disable_keypad(&mut self) -> io::Result<()> {
        todo!()
    }

    fn enable_keypad(&mut self) -> io::Result<bool> {
        todo!()
    }

    fn disable_mouse(&mut self) -> io::Result<()> {
        todo!()
    }

    fn enable_mouse(&mut self, track_motion: bool) -> io::Result<bool> {
        todo!()
    }

    fn enter_screen(&mut self) -> io::Result<()> {
        todo!()
    }

    fn exit_screen(&mut self) -> io::Result<()> {
        todo!()
    }

    pub fn clear_attributes(&mut self) -> io::Result<()> {
        todo!()
    }

    pub fn set_fg(&mut self, fg: Option<Color>) -> io::Result<()> {
        todo!()
    }

    pub fn set_bg(&mut self, bg: Option<Color>) -> io::Result<()> {
        todo!()
    }

    pub fn add_style(&mut self, style: Style) -> io::Result<()> {
        todo!()
    }

    pub fn remove_style(&mut self, style: Style) -> io::Result<()> {
        todo!()
    }

    pub fn set_style(&mut self, style: Style) -> io::Result<()> {
        todo!()
    }

    pub fn set_theme(&mut self, theme: Theme) -> io::Result<()> {
        todo!()
    }

    pub fn set_attrs(&mut self, fg: Option<Color>, bg: Option<Color>, style: Style) -> io::Result<()> {
        todo!()
    }

    fn clear_fg(&mut self) -> io::Result<()> {
        todo!()
    }

    fn clear_bg(&mut self) -> io::Result<()> {
        todo!()
    }

    fn set_fg_color(&mut self, fg: Color) -> io::Result<()> {
        todo!()
    }

    fn set_bg_color(&mut self, bg: Color) -> io::Result<()> {
        todo!()
    }

    pub fn clear_screen(&mut self) -> io::Result<()> {
        todo!()
    }

    pub fn clear_to_line_end(&mut self) -> io::Result<()> {
        todo!()
    }

    pub fn clear_to_screen_end(&mut self) -> io::Result<()> {
        todo!()
    }

    pub fn move_up(&mut self, n: usize) -> io::Result<()> {
        todo!()
    }

    pub fn move_down(&mut self, n: usize) -> io::Result<()> {
        todo!()
    }

    pub fn move_left(&mut self, n: usize) -> io::Result<()> {
        todo!()
    }

    pub fn move_right(&mut self, n: usize) -> io::Result<()> {
        todo!()
    }

    pub fn move_to_first_column(&mut self) -> io::Result<()> {
        todo!()
    }

    pub fn move_cursor(&mut self, pos: Cursor) -> io::Result<()> {
        todo!()
    }

    pub fn set_cursor_mode(&mut self, mode: CursorMode) -> io::Result<()> {
        todo!()
    }

    pub fn write_char(&mut self, ch: char) -> io::Result<()> {
        todo!()
    }

    pub fn write_str(&mut self, s: &str) -> io::Result<()> {
        todo!()
    }

    pub fn write_styled(&mut self,
            fg: Option<Color>, bg: Option<Color>, style: Style, text: &str)
            -> io::Result<()> {
        todo!()
    }

    fn write_bytes(&mut self, buf: &[u8]) -> io::Result<()> {
        todo!()
    }

    pub fn flush(&mut self) -> io::Result<()> {
        todo!()
    }

    fn write_data(&self, buf: &[u8]) -> (usize, io::Result<()>) {
        todo!()
    }
}
