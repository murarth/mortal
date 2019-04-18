use std::io;
use std::sync::{LockResult, Mutex, MutexGuard, TryLockResult};
use std::time::Duration;

use crate::buffer::ScreenBuffer;
use crate::priv_util::{
    map_lock_result, map_try_lock_result,
    map2_lock_result, map2_try_lock_result,
};
use crate::sys::{Terminal, TerminalReadGuard, TerminalWriteGuard, PrepareState};
use crate::terminal::{Color, Cursor, CursorMode, Event, Size, Style, PrepareConfig};

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
        let size = term.size()?;
        let state = term.prepare(config)?;

        let screen = Screen{
            term: term,
            state: Some(state),

            writer: Mutex::new(Writer{
                buffer: ScreenBuffer::new(size),
                clear_screen: false,
                real_cursor: Cursor::default(),
            }),
        };

        screen.term.enter_screen()?;

        Ok(screen)
    }

    pub fn stdout(config: PrepareConfig) -> io::Result<Screen> {
        Screen::new(Terminal::stdout()?, config)
    }

    pub fn stderr(config: PrepareConfig) -> io::Result<Screen> {
        Screen::new(Terminal::stderr()?, config)
    }

    forward_screen_buffer_methods!{ |slf| slf.lock_write_data().buffer }

    pub fn lock_read(&self) -> LockResult<ScreenReadGuard> {
        map_lock_result(self.term.lock_read(),
            |r| ScreenReadGuard::new(self, r))
    }

    pub fn try_lock_read(&self) -> TryLockResult<ScreenReadGuard> {
        map_try_lock_result(self.term.try_lock_read(),
            |r| ScreenReadGuard::new(self, r))
    }

    pub fn lock_write(&self) -> LockResult<ScreenWriteGuard> {
        map2_lock_result(self.term.lock_write(), self.writer.lock(),
            |a, b| ScreenWriteGuard::new(a, b))
    }

    pub fn try_lock_write(&self) -> TryLockResult<ScreenWriteGuard> {
        map2_try_lock_result(self.term.try_lock_write(), self.writer.try_lock(),
            |a, b| ScreenWriteGuard::new(a, b))
    }

    fn lock_reader(&self) -> ScreenReadGuard {
        self.lock_read().expect("Screen::lock_reader")
    }

    fn lock_writer(&self) -> ScreenWriteGuard {
        self.lock_write().expect("Screen::lock_writer")
    }

    fn lock_write_data(&self) -> MutexGuard<Writer> {
        self.writer.lock().expect("Screen::lock_write_data")
    }

    pub fn name(&self) -> &str {
        self.term.name()
    }

    pub fn set_cursor_mode(&self, mode: CursorMode) -> io::Result<()> {
        self.term.set_cursor_mode(mode)
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

    pub fn refresh(&self) -> io::Result<()> {
        self.lock_writer().refresh()
    }
}

impl Drop for Screen {
    fn drop(&mut self) {
        let res = if let Some(state) = self.state.take() {
            self.term.restore(state)
        } else {
            Ok(())
        };

        if let Err(e) = res.and_then(|_| self.term.exit_screen()) {
            eprintln!("failed to restore terminal: {}", e);
        }
    }
}

impl<'a> ScreenReadGuard<'a> {
    fn new(screen: &'a Screen, reader: TerminalReadGuard<'a>) -> ScreenReadGuard<'a> {
        ScreenReadGuard{screen, reader}
    }

    pub fn wait_event(&mut self, timeout: Option<Duration>) -> io::Result<bool> {
        self.reader.wait_event(timeout)
    }

    pub fn read_event(&mut self, timeout: Option<Duration>) -> io::Result<Option<Event>> {
        let r = self.reader.read_event(timeout)?;

        if let Some(Event::Resize(size)) = r {
            self.screen.lock_write_data().update_size(size);
        }

        Ok(r)
    }

    pub fn read_raw(&mut self, buf: &mut [u8], timeout: Option<Duration>) -> io::Result<Option<Event>> {
        let r = self.reader.read_raw(buf, timeout)?;

        if let Some(Event::Resize(size)) = r {
            self.screen.lock_write_data().update_size(size);
        }

        Ok(r)
    }
}

impl<'a> ScreenWriteGuard<'a> {
    fn new(writer: TerminalWriteGuard<'a>, data: MutexGuard<'a, Writer>)
            -> ScreenWriteGuard<'a> {
        ScreenWriteGuard{writer, data}
    }

    forward_screen_buffer_mut_methods!{ |slf| slf.data.buffer }

    pub fn set_cursor_mode(&mut self, mode: CursorMode) -> io::Result<()> {
        self.writer.set_cursor_mode(mode)
    }

    pub fn refresh(&mut self) -> io::Result<()> {
        if self.data.clear_screen {
            self.writer.clear_screen()?;
            self.data.clear_screen = false;
        }

        self.writer.clear_attributes()?;

        let mut indices = self.data.buffer.indices();

        while let Some((pos, cell)) = self.data.buffer.next_cell(&mut indices) {
            self.move_cursor(pos)?;

            self.apply_attrs(cell.attrs())?;
            self.writer.write_str(cell.text())?;
            self.data.real_cursor.column += 1;
        }

        self.writer.clear_attributes()?;

        let size = self.data.buffer.size();
        let pos = self.data.buffer.cursor();

        if pos.is_out_of_bounds(size) {
            self.move_cursor(Cursor::last(size))?;
        } else {
            self.move_cursor(pos)?;
        }

        self.writer.flush()
    }

    fn move_cursor(&mut self, pos: Cursor) -> io::Result<()> {
        if self.data.real_cursor != pos {
            self.writer.move_cursor(pos)?;
            self.data.real_cursor = pos;
        }

        Ok(())
    }

    fn apply_attrs(&mut self,
            (fg, bg, style): (Option<Color>, Option<Color>, Style))
            -> io::Result<()> {
        self.writer.set_attrs(fg, bg, style)
    }
}

impl<'a> Drop for ScreenWriteGuard<'a> {
    fn drop(&mut self) {
        if let Err(e) = self.refresh() {
            eprintln!("failed to refresh screen: {}", e);
        }
    }
}

impl Writer {
    fn update_size(&mut self, new_size: Size) {
        if self.real_cursor.is_out_of_bounds(new_size) {
            // Force cursor move on next refresh
            self.real_cursor = (!0, !0).into();
        }
        self.buffer.resize(new_size);
        self.clear_screen = true;
    }
}
