use std::io;
use std::sync::{LockResult, Mutex, MutexGuard, TryLockResult};
use std::time::Duration;

use crate::buffer::ScreenBuffer;
use crate::priv_util::{
    map2_lock_result, map2_try_lock_result, map_lock_result, map_try_lock_result,
};
use crate::sys::{PrepareState, Terminal, TerminalReadGuard, TerminalWriteGuard};
use crate::terminal::{Color, Cursor, CursorMode, Event, PrepareConfig, Size, Style};

pub struct Screen {
    term: Terminal,

    state: Option<PrepareState>,
    writer: Mutex<Writer>,
}

pub struct ScreenReadGuard<'a> {
    screen: &'a Screen,
    reader: TerminalReadGuard<'a>,
}

pub struct ScreenWriteGuard<'a> {
    writer: TerminalWriteGuard<'a>,
    data: MutexGuard<'a, Writer>,
}

struct Writer {
    buffer: ScreenBuffer,
    clear_screen: bool,
    real_cursor: Cursor,
}

impl Screen {
    pub fn new(term: Terminal, config: PrepareConfig) -> io::Result<Screen> {
        todo!()
    }

    pub fn stdout(config: PrepareConfig) -> io::Result<Screen> {
        Screen::new(Terminal::stdout()?, config)
    }

    pub fn stderr(config: PrepareConfig) -> io::Result<Screen> {
        Screen::new(Terminal::stderr()?, config)
    }

    forward_screen_buffer_methods!{ |slf| slf.lock_write_data().buffer }

    pub fn lock_read(&self) -> LockResult<ScreenReadGuard> {
        todo!()
    }

    pub fn try_lock_read(&self) -> TryLockResult<ScreenReadGuard> {
        todo!()
    }

    pub fn lock_write(&self) -> LockResult<ScreenWriteGuard> {
        todo!()
    }

    pub fn try_lock_write(&self) -> TryLockResult<ScreenWriteGuard> {
        todo!()
    }

    fn lock_reader(&self) -> ScreenReadGuard {
        todo!()
    }

    fn lock_writer(&self) -> ScreenWriteGuard {
        todo!()
    }

    fn lock_write_data(&self) -> MutexGuard<Writer> {
        todo!()
    }

    pub fn name(&self) -> &str {
        todo!()
    }

    pub fn set_cursor_mode(&self, mode: CursorMode) -> io::Result<()> {
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

    pub fn refresh(&self) -> io::Result<()> {
        todo!()
    }
}

impl Drop for Screen {
    fn drop(&mut self) {
        todo!()
    }
}
impl<'a> ScreenReadGuard<'a> {
    fn new(screen: &'a Screen, reader: TerminalReadGuard<'a>) -> ScreenReadGuard<'a> {
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
}

impl<'a> ScreenWriteGuard<'a> {
    fn new(writer: TerminalWriteGuard<'a>, data: MutexGuard<'a, Writer>)
            -> ScreenWriteGuard<'a> {
        todo!()
    }

    forward_screen_buffer_mut_methods!{ |slf| slf.data.buffer }

    pub fn set_cursor_mode(&mut self, mode: CursorMode) -> io::Result<()> {
        todo!()
    }

    pub fn refresh(&mut self) -> io::Result<()> {
        todo!()
    }

    fn move_cursor(&mut self, pos: Cursor) -> io::Result<()> {
        todo!()
    }

    fn apply_attrs(&mut self,
            (fg, bg, style): (Option<Color>, Option<Color>, Style))
            -> io::Result<()> {
        todo!()
    }
}

impl<'a> Drop for ScreenWriteGuard<'a> {
    fn drop(&mut self) {
        todo!()
    }
}

impl Writer {
    fn update_size(&mut self, new_size: Size) {
        todo!()
    }
}
