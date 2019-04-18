//! Provides a drawable buffer on terminal devices

use std::fmt;
use std::io;
use std::sync::{LockResult, TryLockResult};
use std::time::Duration;

use crate::priv_util::{map_lock_result, map_try_lock_result};
use crate::sys;
use crate::terminal::{
    Color, Cursor, CursorMode, Event, PrepareConfig, Size, Style, Theme,
    Terminal,
};

/// Provides operations on an underlying terminal device in screen mode.
///
/// `Screen` uses an internal buffer to store rendered text, colors, and style.
///
/// Each set of changes must be followed by a call to [`refresh`] to flush these
/// changes to the terminal device.
///
/// # Concurrency
///
/// Access to read and write operations is controlled by two internal locks:
/// One for [reading] and one for [writing]. Each lock may be held independently
/// of the other.
///
/// If any one thread wishes to hold both locks, the read lock
/// must be acquired first, in order to prevent deadlocks.
///
/// [`refresh`]: #method.refresh
/// [reading]: struct.ScreenReadGuard.html
/// [writing]: struct.ScreenWriteGuard.html
pub struct Screen(sys::Screen);

/// Holds an exclusive lock for read operations on a `Screen`
///
/// See [`Screen`] documentation for details on locking.
///
/// [`Screen`]: struct.Screen.html
pub struct ScreenReadGuard<'a>(sys::ScreenReadGuard<'a>);

/// Holds an exclusive lock for write operations on a `Screen`
///
/// See [`Screen`] documentation for details on locking.
///
/// [`Screen`]: struct.Screen.html
pub struct ScreenWriteGuard<'a>(sys::ScreenWriteGuard<'a>);

impl Screen {
    /// Opens a new screen interface on `stdout`.
    pub fn new(config: PrepareConfig) -> io::Result<Screen> {
        sys::Screen::stdout(config).map(Screen)
    }

    /// Opens a new screen interface on `stderr`.
    pub fn stderr(config: PrepareConfig) -> io::Result<Screen> {
        sys::Screen::stderr(config).map(Screen)
    }

    /// Begins a new screen session using the given `Terminal` instance.
    pub fn with_terminal(term: Terminal, config: PrepareConfig) -> io::Result<Screen> {
        sys::Screen::new(term.0, config).map(Screen)
    }

    /// Returns the name of the terminal.
    ///
    /// # Notes
    ///
    /// On Unix, this method returns the contents of the `TERM` environment variable.
    ///
    /// On Windows, this method always returns the string `"windows-console"`.
    #[inline]
    pub fn name(&self) -> &str {
        self.0.name()
    }

    /// Attempts to acquire an exclusive lock on terminal read operations.
    ///
    /// The current thread will block until the lock can be acquired.
    #[inline]
    pub fn lock_read(&self) -> LockResult<ScreenReadGuard> {
        map_lock_result(self.0.lock_read(), ScreenReadGuard)
    }

    /// Attempts to acquire an exclusive lock on terminal write operations.
    ///
    /// The current thread will block until the lock can be acquired.
    #[inline]
    pub fn lock_write(&self) -> LockResult<ScreenWriteGuard> {
        map_lock_result(self.0.lock_write(), ScreenWriteGuard)
    }

    /// Attempts to acquire an exclusive lock on terminal read operations.
    ///
    /// If the lock cannot be acquired immediately, `Err(_)` is returned.
    #[inline]
    pub fn try_lock_read(&self) -> TryLockResult<ScreenReadGuard> {
        map_try_lock_result(self.0.try_lock_read(), ScreenReadGuard)
    }

    /// Attempts to acquire an exclusive lock on terminal write operations.
    ///
    /// If the lock cannot be acquired immediately, `Err(_)` is returned.
    #[inline]
    pub fn try_lock_write(&self) -> TryLockResult<ScreenWriteGuard> {
        map_try_lock_result(self.0.try_lock_write(), ScreenWriteGuard)
    }
}

/// # Locking
///
/// The following methods internally acquire the read lock.
///
/// The lock is released before the method returns.
///
/// These methods are also implemented on [`ScreenReadGuard`],
/// which holds the `Screen` read lock until the value is dropped.
///
/// [`ScreenReadGuard`]: struct.ScreenReadGuard.html
impl Screen {
    /// Waits for an event from the terminal.
    ///
    /// Returns `Ok(false)` if `timeout` elapses without an event occurring.
    ///
    /// If `timeout` is `None`, this method will wait indefinitely.
    ///
    /// # Notes
    ///
    /// Some low-level terminal events may not generate an `Event` value.
    /// Therefore, this method may return `Ok(true)`, while a follow-up call
    /// to `read_event` may not immediately return an event.
    pub fn wait_event(&self, timeout: Option<Duration>) -> io::Result<bool> {
        self.0.wait_event(timeout)
    }

    /// Reads an event from the terminal.
    ///
    /// If `timeout` elapses without an event occurring, this method will return
    /// `Ok(None)`.
    ///
    /// If `timeout` is `None`, this method will wait indefinitely.
    pub fn read_event(&self, timeout: Option<Duration>) -> io::Result<Option<Event>>  {
        self.0.read_event(timeout)
    }
}

/// # Locking
///
/// The following methods internally acquire the write lock.
///
/// The lock is released before the method returns.
///
/// These methods are also implemented on [`ScreenWriteGuard`],
/// which holds the `Screen` write lock until the value is dropped.
///
/// [`ScreenWriteGuard`]: struct.ScreenWriteGuard.html
impl Screen {
    /// Returns the current size of the terminal screen.
    #[inline]
    pub fn size(&self) -> Size {
        self.0.size()
    }

    /// Returns the current cursor position.
    #[inline]
    pub fn cursor(&self) -> Cursor {
        self.0.cursor()
    }

    /// Sets the cursor position.
    #[inline]
    pub fn set_cursor<C: Into<Cursor>>(&self, pos: C) {
        self.0.set_cursor(pos.into());
    }

    /// Moves the cursor to the given column on the next line.
    #[inline]
    pub fn next_line(&self, column: usize) {
        self.0.next_line(column);
    }

    /// Set the current cursor mode.
    ///
    /// This setting is a visible hint to the user.
    /// It produces no change in behavior.
    ///
    /// # Notes
    ///
    /// On Unix systems, this setting may have no effect.
    pub fn set_cursor_mode(&self, mode: CursorMode) -> io::Result<()> {
        self.0.set_cursor_mode(mode)
    }

    /// Clears the internal screen buffer.
    pub fn clear_screen(&self) {
        self.0.clear_screen();
    }

    /// Adds a set of `Style` flags to the current style setting.
    #[inline]
    pub fn add_style(&self, style: Style) {
        self.0.add_style(style);
    }

    /// Removes a set of `Style` flags to the current style setting.
    #[inline]
    pub fn remove_style(&self, style: Style) {
        self.0.remove_style(style);
    }

    /// Sets the current style setting to the given set of flags.
    #[inline]
    pub fn set_style<S: Into<Option<Style>>>(&self, style: S) {
        self.0.set_style(style.into().unwrap_or_default());
    }

    /// Sets or removes foreground text color.
    #[inline]
    pub fn set_fg<C: Into<Option<Color>>>(&self, fg: C) {
        self.0.set_fg(fg.into());
    }

    /// Sets or removes background text color.
    #[inline]
    pub fn set_bg<C: Into<Option<Color>>>(&self, bg: C) {
        self.0.set_bg(bg.into());
    }

    /// Sets all attributes for the screen.
    #[inline]
    pub fn set_theme(&self, theme: Theme) {
        self.0.set_theme(theme)
    }

    /// Removes color and style attributes.
    #[inline]
    pub fn clear_attributes(&self) {
        self.0.clear_attributes();
    }

    /// Adds bold to the current style setting.
    ///
    /// This is equivalent to `self.add_style(Style::BOLD)`.
    #[inline]
    pub fn bold(&self) {
        self.add_style(Style::BOLD);
    }

    /// Adds italic to the current style setting.
    ///
    /// This is equivalent to `self.add_style(Style::ITALIC)`.
    #[inline]
    pub fn italic(&self) {
        self.add_style(Style::ITALIC);
    }

    /// Adds underline to the current style setting.
    ///
    /// This is equivalent to `self.add_style(Style::UNDERLINE)`.
    #[inline]
    pub fn underline(&self) {
        self.add_style(Style::UNDERLINE);
    }

    /// Adds reverse to the current style setting.
    ///
    /// This is equivalent to `self.add_style(Style::REVERSE)`.
    #[inline]
    pub fn reverse(&self) {
        self.add_style(Style::REVERSE);
    }

    /// Renders the internal buffer to the terminal screen.
    pub fn refresh(&self) -> io::Result<()> {
        self.0.refresh()
    }

    /// Writes text at the given position within the screen buffer.
    ///
    /// Any non-printable characters, such as escape sequences, will be ignored.
    pub fn write_at<C>(&self, position: C, text: &str)
            where C: Into<Cursor> {
        self.0.write_at(position.into(), text);
    }

    /// Writes text with the given attributes at the current cursor position.
    ///
    /// Any non-printable characters, such as escape sequences, will be ignored.
    pub fn write_styled<F, B, S>(&self, fg: F, bg: B, style: S, text: &str) where
            F: Into<Option<Color>>,
            B: Into<Option<Color>>,
            S: Into<Option<Style>>,
            {
        self.0.write_styled(fg.into(), bg.into(), style.into().unwrap_or_default(), text);
    }

    /// Writes text with the given attributes at the given position within
    /// the screen buffer.
    ///
    /// Any non-printable characters, such as escape sequences, will be ignored.
    pub fn write_styled_at<C, F, B, S>(&self, position: C,
            fg: F, bg: B, style: S, text: &str) where
            C: Into<Cursor>,
            F: Into<Option<Color>>,
            B: Into<Option<Color>>,
            S: Into<Option<Style>>,
            {
        self.0.write_styled_at(position.into(),
            fg.into(), bg.into(), style.into().unwrap_or_default(), text);
    }

    /// Writes a single character at the cursor position
    /// using the current style and color settings.
    ///
    /// If the character is a non-printable character, it will be ignored.
    pub fn write_char(&self, ch: char) {
        self.0.write_char(ch);
    }

    /// Writes a string at the cursor position
    /// using the current style and color settings.
    ///
    /// Any non-printable characters, such as escape sequences, will be ignored.
    pub fn write_str(&self, s: &str) {
        self.0.write_str(s);
    }

    /// Writes formatted text at the cursor position
    /// using the current style and color settings.
    ///
    /// This method enables `Screen` to be used as the receiver to
    /// the [`write!`] and [`writeln!`] macros.
    ///
    /// Any non-printable characters, such as escape sequences, will be ignored.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use std::io;
    /// # use mortal::Screen;
    /// # fn example() -> io::Result<()> {
    /// let screen = Screen::new(Default::default())?;
    ///
    /// writeln!(screen, "Hello, world!");
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// [`write!`]: https://doc.rust-lang.org/std/macro.write.html
    /// [`writeln!`]: https://doc.rust-lang.org/std/macro.writeln.html
    pub fn write_fmt(&self, args: fmt::Arguments) {
        let s = args.to_string();
        self.write_str(&s)
    }

    #[doc(hidden)]
    pub fn borrow_term_write_guard(&self) -> ScreenWriteGuard {
        self.lock_write().unwrap()
    }
}

impl<'a> ScreenReadGuard<'a> {
    /// Waits for an event from the terminal.
    ///
    /// Returns `Ok(false)` if `timeout` elapses without an event occurring.
    ///
    /// If `timeout` is `None`, this method will wait indefinitely.
    ///
    /// # Notes
    ///
    /// Some low-level terminal events may not generate an `Event` value.
    /// Therefore, this method may return `Ok(true)`, while a follow-up call
    /// to `read_event` may not immediately return an event.
    pub fn wait_event(&mut self, timeout: Option<Duration>) -> io::Result<bool> {
        self.0.wait_event(timeout)
    }

    /// Reads an event from the terminal.
    ///
    /// If `timeout` elapses without an event occurring, this method will return
    /// `Ok(None)`.
    ///
    /// If `timeout` is `None`, this method will wait indefinitely.
    pub fn read_event(&mut self, timeout: Option<Duration>) -> io::Result<Option<Event>> {
        self.0.read_event(timeout)
    }
}

impl<'a> ScreenWriteGuard<'a> {
    /// Returns the current size of the terminal screen.
    #[inline]
    pub fn size(&self) -> Size {
        self.0.size()
    }

    /// Sets the cursor position.
    #[inline]
    pub fn cursor(&self) -> Cursor {
        self.0.cursor()
    }

    /// Moves the cursor to the given column on the next line.
    #[inline]
    pub fn set_cursor<C: Into<Cursor>>(&mut self, pos: C) {
        self.0.set_cursor(pos.into());
    }

    /// Set the current cursor mode.
    #[inline]
    pub fn next_line(&mut self, column: usize) {
        self.0.next_line(column);
    }

    /// Set the current cursor mode.
    ///
    /// This setting is a visible hint to the user.
    /// It produces no change in behavior.
    ///
    /// # Notes
    ///
    /// On Unix systems, this setting may have no effect.
    pub fn set_cursor_mode(&mut self, mode: CursorMode) -> io::Result<()> {
        self.0.set_cursor_mode(mode)
    }

    /// Adds a set of `Style` flags to the current style setting.
    pub fn clear_screen(&mut self) {
        self.0.clear_screen();
    }

    /// Removes a set of `Style` flags to the current style setting.
    /// Adds a set of `Style` flags to the current style setting.
    #[inline]
    pub fn add_style(&mut self, style: Style) {
        self.0.add_style(style)
    }

    /// Sets the current style setting to the given set of flags.
    #[inline]
    pub fn remove_style(&mut self, style: Style) {
        self.0.remove_style(style)
    }

    /// Sets or removes foreground text color.
    #[inline]
    pub fn set_style<S: Into<Option<Style>>>(&mut self, style: S) {
        self.0.set_style(style.into().unwrap_or_default())
    }

    /// Sets or removes background text color.
    #[inline]
    pub fn set_fg<C: Into<Option<Color>>>(&mut self, fg: C) {
        self.0.set_fg(fg.into())
    }

    /// Removes color and style attributes.
    #[inline]
    pub fn set_bg<C: Into<Option<Color>>>(&mut self, bg: C) {
        self.0.set_bg(bg.into())
    }

    /// Sets all attributes for the screen.
    #[inline]
    pub fn set_theme(&mut self, theme: Theme) {
        self.0.set_theme(theme)
    }

    /// Adds bold to the current style setting.
    #[inline]
    pub fn clear_attributes(&mut self) {
        self.0.clear_attributes()
    }

    /// Adds bold to the current style setting.
    ///
    /// This is equivalent to `self.add_style(Style::BOLD)`.
    #[inline]
    pub fn bold(&mut self) {
        self.add_style(Style::BOLD)
    }

    /// Adds italic to the current style setting.
    ///
    /// This is equivalent to `self.add_style(Style::ITALIC)`.
    #[inline]
    pub fn italic(&mut self) {
        self.add_style(Style::ITALIC);
    }

    /// Adds underline to the current style setting.
    ///
    /// This is equivalent to `self.add_style(Style::UNDERLINE)`.
    #[inline]
    pub fn underline(&mut self) {
        self.add_style(Style::UNDERLINE)
    }

    /// Adds reverse to the current style setting.
    ///
    /// This is equivalent to `self.add_style(Style::REVERSE)`.
    #[inline]
    pub fn reverse(&mut self) {
        self.add_style(Style::REVERSE)
    }

    /// Renders the internal buffer to the terminal screen.
    ///
    /// This is called automatically when the `ScreenWriteGuard` is dropped.
    pub fn refresh(&mut self) -> io::Result<()> {
        self.0.refresh()
    }

    /// Writes text at the given position within the screen buffer.
    ///
    /// Any non-printable characters, such as escape sequences, will be ignored.
    pub fn write_at<C>(&mut self, position: C, text: &str)
            where C: Into<Cursor> {
        self.0.write_at(position.into(), text)
    }

    /// Writes text with the given attributes at the current cursor position.
    ///
    /// Any non-printable characters, such as escape sequences, will be ignored.
    pub fn write_styled<F, B, S>(&mut self, fg: F, bg: B, style: S, text: &str) where
            F: Into<Option<Color>>,
            B: Into<Option<Color>>,
            S: Into<Option<Style>>,
            {
        self.0.write_styled(fg.into(), bg.into(), style.into().unwrap_or_default(), text)
    }

    /// Writes text with the given attributes at the given position within
    /// the screen buffer.
    ///
    /// Any non-printable characters, such as escape sequences, will be ignored.
    pub fn write_styled_at<C, F, B, S>(&mut self, position: C,
            fg: F, bg: B, style: S, text: &str) where
            C: Into<Cursor>,
            F: Into<Option<Color>>,
            B: Into<Option<Color>>,
            S: Into<Option<Style>>,
            {
        self.0.write_styled_at(position.into(),
            fg.into(), bg.into(), style.into().unwrap_or_default(), text)
    }

    /// Writes a single character at the cursor position
    /// using the current style and color settings.
    ///
    /// If the character is a non-printable character, it will be ignored.
    pub fn write_char(&mut self, ch: char) {
        self.0.write_char(ch)
    }

    /// Writes a string at the cursor position
    /// using the current style and color settings.
    ///
    /// Any non-printable characters, such as escape sequences, will be ignored.
    pub fn write_str(&mut self, s: &str) {
        self.0.write_str(s)
    }

    /// Writes formatted text at the cursor position
    /// using the current style and color settings.
    ///
    /// This method enables `ScreenWriteGuard` to be used as the receiver to
    /// the [`write!`] and [`writeln!`] macros.
    ///
    /// Any non-printable characters, such as escape sequences, will be ignored.
    ///
    /// [`write!`]: https://doc.rust-lang.org/std/macro.write.html
    /// [`writeln!`]: https://doc.rust-lang.org/std/macro.writeln.html
    pub fn write_fmt(&mut self, args: fmt::Arguments) {
        let s = args.to_string();
        self.write_str(&s)
    }

    #[doc(hidden)]
    pub fn borrow_term_write_guard(&mut self) -> &mut Self {
        self
    }
}

#[cfg(unix)]
impl crate::unix::TerminalExt for Screen {
    fn read_raw(&mut self, buf: &mut [u8], timeout: Option<Duration>) -> io::Result<Option<Event>> {
        self.0.read_raw(buf, timeout)
    }
}

#[cfg(unix)]
impl<'a> crate::unix::TerminalExt for ScreenReadGuard<'a> {
    fn read_raw(&mut self, buf: &mut [u8], timeout: Option<Duration>) -> io::Result<Option<Event>> {
        self.0.read_raw(buf, timeout)
    }
}

#[cfg(windows)]
impl crate::windows::TerminalExt for Screen {
    fn read_raw(&mut self, buf: &mut [u16], timeout: Option<Duration>) -> io::Result<Option<Event>> {
        self.0.read_raw(buf, timeout)
    }

    fn read_raw_event(&mut self, events: &mut [::winapi::um::wincon::INPUT_RECORD],
            timeout: Option<Duration>) -> io::Result<Option<Event>> {
        self.0.read_raw_event(events, timeout)
    }
}

#[cfg(windows)]
impl<'a> crate::windows::TerminalExt for ScreenReadGuard<'a> {
    fn read_raw(&mut self, buf: &mut [u16], timeout: Option<Duration>) -> io::Result<Option<Event>> {
        self.0.read_raw(buf, timeout)
    }

    fn read_raw_event(&mut self, events: &mut [::winapi::um::wincon::INPUT_RECORD],
            timeout: Option<Duration>) -> io::Result<Option<Event>> {
        self.0.read_raw_event(events, timeout)
    }
}
