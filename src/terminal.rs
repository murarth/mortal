//! Provides an interface to terminal devices

use std::fmt;
use std::io;
use std::sync::{LockResult, TryLockResult};
use std::time::Duration;

use crate::priv_util::{map_lock_result, map_try_lock_result};
use crate::signal::{Signal, SignalSet};
use crate::sys;

/// Represents a color attribute applied to text foreground or background.
///
/// # Notes
///
/// Names here correspond to possible default values for some systems.
/// Because users may reconfigure the set of colors available in their terminal,
/// these color values may correspond to different user-configured display colors.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum Color {
    /// Black
    Black,
    /// Blue
    Blue,
    /// Cyan
    Cyan,
    /// Green
    Green,
    /// Magenta
    Magenta,
    /// Red
    Red,
    /// White
    White,
    /// Yellow
    Yellow,
}

bitflags!{
    /// Represents a set of style attributes applied to text.
    ///
    /// Some styles may not be supported on all systems.
    #[derive(Copy, Clone, Debug, Default, Eq, PartialEq, Hash)]
    pub struct Style: u8 {
        /// Bold
        const BOLD      = 1 << 0;
        /// Italic
        const ITALIC    = 1 << 1;
        /// Reverse; foreground and background color swapped
        const REVERSE   = 1 << 2;
        /// Underline
        const UNDERLINE = 1 << 3;
    }
}

/// Represents a terminal output theme.
///
/// A theme consists of a foreground and background color as well as a style.
#[derive(Copy, Clone, Debug, Default)]
pub struct Theme {
    /// Foreground color
    pub fg: Option<Color>,
    /// Background color
    pub bg: Option<Color>,
    /// Style
    pub style: Style,
}

impl Theme {
    /// Creates a new theme with given values.
    ///
    /// # Note
    ///
    /// In order to create a Theme using default values you might want to use
    /// `Theme::default()` instead.
    pub fn new<F,B,S>(fg: F, bg: B, style: S) -> Theme
            where
                F: Into<Option<Color>>,
                B: Into<Option<Color>>,
                S: Into<Option<Style>> {
        Theme {
            fg: fg.into(),
            bg: bg.into(),
            style: style.into().unwrap_or_default(),
        }
    }

    /// Sets the foreground color on the given Theme and returns the new.
    pub fn fg<F>(mut self, fg: F) -> Theme
            where F: Into<Option<Color>> {
        self.fg = fg.into();
        self
    }

    /// Sets the background color on the given Theme and returns the new.
    pub fn bg<B>(mut self, bg: B) -> Theme
            where B: Into<Option<Color>> {
        self.bg = bg.into();
        self
    }

    /// Sets the style on the given Theme and returns the new.
    pub fn style<S>(mut self, style: S) -> Theme
            where S: Into<Option<Style>> {
        self.style = style.into().unwrap_or_default();
        self
    }
}

/// Represents the cursor position in a terminal device
#[derive(Copy, Clone, Debug, Default, Eq, PartialEq)]
pub struct Cursor {
    /// Index of line in terminal, beginning at `0`.
    pub line: usize,
    /// Index of column in terminal, beginning at `0`.
    pub column: usize,
}

impl Cursor {
    /// Returns the position of the next cell within a terminal of the given size.
    ///
    /// Returns `None` if this cursor position represents the last cell.
    #[inline]
    pub fn next(&self, size: Size) -> Option<Cursor> {
        let mut line = self.line;
        let mut column = self.column + 1;

        if column >= size.columns {
            column = 0;
            line += 1;
        }

        if line >= size.lines {
            None
        } else {
            Some(Cursor{line, column})
        }
    }

    /// Returns the position of the previous cell within a terminal of the given size.
    ///
    /// Returns `None` if this cursor position represents the first cell.
    #[inline]
    pub fn previous(&self, size: Size) -> Option<Cursor> {
        if self.column == 0 {
            if self.line == 0 {
                None
            } else {
                Some(Cursor{line: self.line - 1, column: size.columns - 1})
            }
        } else {
            Some(Cursor{line: self.line, column: self.column - 1})
        }
    }

    /// Returns a `Cursor` pointing to the first cell, i.e. `(0, 0)`.
    #[inline]
    pub fn first() -> Cursor {
        Cursor{
            line: 0,
            column: 0,
        }
    }

    /// Returns a `Cursor` pointing to the last cell of a screen of the given size.
    #[inline]
    pub fn last(size: Size) -> Cursor {
        Cursor{
            line: size.lines - 1,
            column: size.columns - 1,
        }
    }

    /// Returns whether the cursor is out of bounds of the given size.
    #[inline]
    pub fn is_out_of_bounds(&self, size: Size) -> bool {
        self.line >= size.lines || self.column >= size.columns
    }

    /// Returns the index of the cursor position within a one-dimensional array
    /// of the given size.
    pub(crate) fn as_index(&self, size: Size) -> usize {
        self.line * size.columns + self.column
    }
}

impl From<(usize, usize)> for Cursor {
    /// Returns a `Cursor` value from a `(line, column)` or `(y, x)` tuple.
    fn from((line, column): (usize, usize)) -> Cursor {
        Cursor{line, column}
    }
}

/// Represents the visual appearance of the cursor in the terminal
///
/// Some cursor states may not be available on all systems.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum CursorMode {
    /// Normal mode
    Normal,
    /// Invisible mode
    Invisible,
    /// Overwrite mode
    Overwrite,
}

/// Represents an event generated from a terminal interface
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum Event {
    /// Keyboard event
    Key(Key),
    /// Mouse event
    Mouse(MouseEvent),
    /// Raw data read
    ///
    /// A value of this variant can only be returned when using the
    /// platform-dependent extension trait, `TerminalExt`.
    ///
    /// On Unix, this trait is [`mortal::unix::TerminalExt`].
    ///
    /// On Windows, this trait is [`mortal::windows::TerminalExt`].
    ///
    /// [`mortal::unix::TerminalExt`]: ../unix/trait.TerminalExt.html
    /// [`mortal::windows::TerminalExt`]: ../windows/trait.TerminalExt.html
    Raw(usize),
    /// Terminal window size changed; contained value is the new size.
    Resize(Size),
    /// Terminal signal received
    Signal(Signal),
    /// No event
    ///
    /// Returned when a low-level terminal event does not correspond
    /// to a reported event type.
    NoEvent,
}

/// Represents a keyboard key press event
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum Key {
    /// Backspace
    Backspace,
    /// Enter
    Enter,
    /// Escape
    Escape,
    /// Tab
    Tab,
    /// Up arrow
    Up,
    /// Down arrow
    Down,
    /// Left arrow
    Left,
    /// Right arrow
    Right,
    /// Delete
    Delete,
    /// Insert
    Insert,
    /// Home
    Home,
    /// End
    End,
    /// PageUp
    PageUp,
    /// PageDown
    PageDown,
    /// Character key
    Char(char),
    /// Control character
    ///
    /// # Notes
    ///
    /// The contained `char` value must always be lowercase;
    /// e.g. `Ctrl('a')` and **not** `Ctrl('A')`.
    ///
    /// On Unix, certain special `Key` values are represented as control
    /// characters; therefore, the following combinations will not generate a
    /// `Ctrl(_)` value:
    ///
    /// * Ctrl-I instead generates `Tab`
    /// * Ctrl-J and Ctrl-M instead generate `Enter`
    Ctrl(char),
    /// Function `n` key; e.g. F1, F2, ...
    F(u32),
}

impl From<char> for Key {
    fn from(ch: char) -> Key {
        use crate::util::{is_ctrl, unctrl_lower};

        match ch {
            '\x1b' => Key::Escape,
            '\x7f' => Key::Backspace,
            '\r' | '\n' => Key::Enter,
            '\t' => Key::Tab,
            _ if is_ctrl(ch) => Key::Ctrl(unctrl_lower(ch)),
            _ => Key::Char(ch),
        }
    }
}

/// Represents a mouse event
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct MouseEvent {
    /// The position of the mouse within the terminal when the event occurred
    pub position: Cursor,
    /// The input event that occurred
    pub input: MouseInput,
    /// Modifier keys held when the input event occurred
    ///
    /// # Notes
    ///
    /// On some systems, certain combinations of mouse button and modifier may
    /// be interpreted by the system and not reported as terminal events.
    pub modifiers: ModifierState,
}

/// Represents the type of mouse input event
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum MouseInput {
    /// The mouse cursor was moved
    Motion,
    /// A mouse button was pressed
    ButtonPressed(MouseButton),
    /// A mouse button was released
    ButtonReleased(MouseButton),
    /// The mouse wheel was scrolled up
    WheelUp,
    /// The mouse wheel was scrolled down
    WheelDown,
}

/// Represents a button on a mouse device
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum MouseButton {
    /// Left mouse button
    Left,
    /// Right mouse button
    Right,
    /// Middle mouse button
    Middle,
    /// Other mouse button
    Other(u32),
}

bitflags!{
    /// Represents a set of modifier keys
    #[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
    pub struct ModifierState: u8 {
        /// Alt key
        const ALT   = 1 << 0;
        /// Ctrl key
        const CTRL  = 1 << 1;
        /// Shift key
        const SHIFT = 1 << 2;
    }
}

/// Configures a [`Terminal`] or [`Screen`] instance to read special input.
///
/// This struct implements the [`Default`] trait, providing default
/// values for all options.
///
/// To override only some options while using the remaining default values,
/// one may use the following construct:
///
/// ```no_run
/// # use std::io;
/// # fn example() -> io::Result<()> {
/// use mortal::{Terminal, PrepareConfig};
///
/// let term = Terminal::new()?;
///
/// let state = term.prepare(PrepareConfig{
///     enable_keypad: false,
///     enable_mouse: true,
///     .. PrepareConfig::default()
/// })?;
///
/// // ...
///
/// term.restore(state)?;
/// # Ok(())
/// # }
/// ```
///
/// [`Default`]: https://doc.rust-lang.org/std/default/trait.Default.html
/// [`Terminal`]: struct.Terminal.html
/// [`Screen`]: ../screen/struct.Screen.html
#[derive(Copy, Clone, Debug)]
pub struct PrepareConfig {
    /// Whether to block signals that result from user input.
    ///
    /// If `true`, e.g. when the user presses Ctrl-C,
    /// `Key(Ctrl('c'))` will be read instead of `Signal(Interrupt)`.
    ///
    /// The default is `true`.
    pub block_signals: bool,
    /// Whether to enable control flow characters.
    ///
    /// The default is `false`.
    ///
    /// # Notes
    ///
    /// On Unix, when this setting is enabled, Ctrl-S and Ctrl-Q
    /// will stop and start, respectively, terminal input from being processed.
    ///
    /// On Windows, this setting has no effect.
    pub enable_control_flow: bool,
    /// If `true`, the terminal will be configured to generate events from
    /// function keys.
    ///
    /// The default is `true`.
    ///
    /// # Notes
    ///
    /// On Unix, this may be required to receive events for arrow keys.
    ///
    /// On Windows, this setting has no effect.
    pub enable_keypad: bool,
    /// If `true`, the terminal will be configured to generate events for
    /// mouse input, if supported, and `read_event` may return `Event::Mouse(_)`.
    ///
    /// The default is `false`.
    ///
    /// # Notes
    ///
    /// This setting may not be supported on all systems.
    pub enable_mouse: bool,
    /// If `true`, mouse motion events will always be reported.
    /// If `false`, such events will only be reported while at least one mouse
    /// button is pressed.
    ///
    /// Mouse events are only reported if `enable_mouse` is `true`.
    ///
    /// The default is `false`.
    pub always_track_motion: bool,
    /// For each signal in the set, a signal handler will intercept the signal
    /// and report it by returning an `Event::Signal(_)` value.
    ///
    /// `block_signals` must be `false` for any of these signals to be received.
    ///
    /// By default, no signals are reported.
    pub report_signals: SignalSet,
}

impl Default for PrepareConfig {
    fn default() -> PrepareConfig {
        PrepareConfig{
            block_signals: true,
            enable_control_flow: false,
            enable_keypad: true,
            enable_mouse: false,
            always_track_motion: false,
            report_signals: SignalSet::new(),
        }
    }
}

/// Represents a previous device state of a [`Terminal`].
///
/// A value of this type is returned by [`Terminal::prepare`].
///
/// Required to revert terminal state using [`Terminal::restore`].
///
/// [`Terminal`]: struct.Terminal.html
/// [`Terminal::prepare`]: struct.Terminal.html#method.prepare
/// [`Terminal::restore`]: struct.Terminal.html#method.restore
#[must_use = "the result of `terminal.prepare()` should be passed to \
    `terminal.restore()` to restore terminal to its original state"]
pub struct PrepareState(sys::PrepareState);

/// Represents the size of a terminal window
///
/// A valid size must not have zero lines or zero columns.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct Size {
    /// Number of lines in the terminal
    pub lines: usize,
    /// Number of columns in the terminal
    pub columns: usize,
}

impl Size {
    /// Returns the total number of cells in a terminal of the given size.
    ///
    /// # Panics
    ///
    /// If `lines * columns` would overflow.
    #[inline]
    pub fn area(&self) -> usize {
        self.checked_area().unwrap_or_else(
            || panic!("overflow in Size::area {:?}", self))
    }

    /// Returns the total number of cells in a terminal of the given size.
    ///
    /// Returns `None` in case of overflow.
    #[inline]
    pub fn checked_area(&self) -> Option<usize> {
        self.lines.checked_mul(self.columns)
    }
}

/// Provides concurrent read and write access to a terminal device
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
/// [reading]: struct.TerminalReadGuard.html
/// [writing]: struct.TerminalWriteGuard.html
pub struct Terminal(pub(crate) sys::Terminal);

/// Holds an exclusive lock for read operations on a `Terminal`
///
/// See [`Terminal`] documentation for details on locking.
///
/// [`Terminal`]: struct.Terminal.html
pub struct TerminalReadGuard<'a>(sys::TerminalReadGuard<'a>);

/// Holds an exclusive lock for write operations on a `Terminal`
///
/// See [`Terminal`] documentation for details on locking.
///
/// [`Terminal`]: struct.Terminal.html
pub struct TerminalWriteGuard<'a>(sys::TerminalWriteGuard<'a>);

impl Terminal {
    /// Opens a new interface to the terminal on `stdout`.
    pub fn new() -> io::Result<Terminal> {
        Ok(Terminal(sys::Terminal::stdout()?))
    }

    /// Opens a new interface to the terminal on `stderr`.
    pub fn stderr() -> io::Result<Terminal> {
        Ok(Terminal(sys::Terminal::stderr()?))
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
    pub fn lock_read(&self) -> LockResult<TerminalReadGuard> {
        map_lock_result(self.0.lock_read(), TerminalReadGuard)
    }

    /// Attempts to acquire an exclusive lock on terminal write operations.
    ///
    /// The current thread will block until the lock can be acquired.
    #[inline]
    pub fn lock_write(&self) -> LockResult<TerminalWriteGuard> {
        map_lock_result(self.0.lock_write(), TerminalWriteGuard)
    }

    /// Attempts to acquire an exclusive lock on terminal read operations.
    ///
    /// If the lock cannot be acquired immediately, `Err(_)` is returned.
    #[inline]
    pub fn try_lock_read(&self) -> TryLockResult<TerminalReadGuard> {
        map_try_lock_result(self.0.try_lock_read(), TerminalReadGuard)
    }

    /// Attempts to acquire an exclusive lock on terminal write operations.
    ///
    /// If the lock cannot be acquired immediately, `Err(_)` is returned.
    #[inline]
    pub fn try_lock_write(&self) -> TryLockResult<TerminalWriteGuard> {
        map_try_lock_result(self.0.try_lock_write(), TerminalWriteGuard)
    }
}

/// # Locking
///
/// The following methods internally acquire both the read and write locks.
///
/// The locks are released before the method returns.
///
/// These methods are also implemented on [`TerminalReadGuard`],
/// which holds the `Terminal` read lock until the value is dropped.
///
/// [`TerminalReadGuard`]: struct.TerminalReadGuard.html
impl Terminal {
    /// Prepares the terminal to read input.
    ///
    /// When reading operations have concluded, [`restore`] should be called
    /// with the resulting `PrepareState` value to restore the terminal to
    /// its previous state.
    ///
    /// This method may be called more than once before a corresponding
    /// `restore` call. However, each `restore` call must receive the most
    /// recently created `PrepareState` value.
    ///
    /// See [`PrepareConfig`] for details.
    ///
    /// [`PrepareConfig`]: struct.PrepareConfig.html
    /// [`restore`]: #method.restore
    pub fn prepare(&self, config: PrepareConfig) -> io::Result<PrepareState> {
        self.0.prepare(config).map(PrepareState)
    }

    /// Restores the terminal to its previous state.
    pub fn restore(&self, state: PrepareState) -> io::Result<()> {
        self.0.restore(state.0)
    }
}

/// # Locking
///
/// The following methods internally acquire the read lock.
///
/// The lock is released before the method returns.
///
/// These methods are also implemented on [`TerminalReadGuard`],
/// which holds the `Terminal` read lock until the value is dropped.
///
/// [`TerminalReadGuard`]: struct.TerminalReadGuard.html
impl Terminal {
    /// Waits for an event from the terminal.
    ///
    /// Returns `Ok(false)` if `timeout` elapses without an event occurring.
    ///
    /// If `timeout` is `None`, this method will wait indefinitely.
    pub fn wait_event(&self, timeout: Option<Duration>) -> io::Result<bool> {
        self.0.wait_event(timeout)
    }

    /// Waits for input and reads an event from the terminal.
    ///
    /// Returns `Ok(None)` if `timeout` elapses without an event occurring.
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
/// These methods are also implemented on [`TerminalWriteGuard`],
/// which holds the `Terminal` write lock until the value is dropped.
///
/// [`TerminalWriteGuard`]: struct.TerminalWriteGuard.html
impl Terminal {
    /// Returns the size of the terminal.
    #[inline]
    pub fn size(&self) -> io::Result<Size> {
        self.0.size()
    }

    /// Clears the terminal screen, placing the cursor at the first line and column.
    ///
    /// If the terminal contains a scrolling window over a buffer, the window
    /// will be scrolled downward, preserving as much of the existing buffer
    /// as possible.
    pub fn clear_screen(&self) -> io::Result<()> {
        self.0.clear_screen()
    }

    /// Clears the current line, starting at cursor position.
    pub fn clear_to_line_end(&self) -> io::Result<()> {
        self.0.clear_to_line_end()
    }

    /// Clears the screen, starting at cursor position.
    pub fn clear_to_screen_end(&self) -> io::Result<()> {
        self.0.clear_to_screen_end()
    }

    /// Moves the cursor up `n` lines.
    pub fn move_up(&self, n: usize) -> io::Result<()> {
        self.0.move_up(n)
    }

    /// Moves the cursor down `n` lines.
    pub fn move_down(&self, n: usize) -> io::Result<()> {
        self.0.move_down(n)
    }

    /// Moves the cursor left `n` columns.
    pub fn move_left(&self, n: usize) -> io::Result<()> {
        self.0.move_left(n)
    }

    /// Moves the cursor right `n` columns.
    pub fn move_right(&self, n: usize) -> io::Result<()> {
        self.0.move_right(n)
    }

    /// Moves the cursor to the first column of the current line
    pub fn move_to_first_column(&self) -> io::Result<()> {
        self.0.move_to_first_column()
    }

    /// Set the current cursor mode.
    ///
    /// This setting is a visible hint to the user.
    /// It produces no change in behavior.
    ///
    /// # Notes
    ///
    /// On some systems, this setting may have no effect.
    pub fn set_cursor_mode(&self, mode: CursorMode) -> io::Result<()> {
        self.0.set_cursor_mode(mode)
    }

    /// Adds a set of `Style` flags to the current style setting.
    pub fn add_style(&self, style: Style) -> io::Result<()> {
        self.0.add_style(style)
    }

    /// Removes a set of `Style` flags from the current style setting.
    pub fn remove_style(&self, style: Style) -> io::Result<()> {
        self.0.remove_style(style)
    }

    /// Sets the current style to the given set of flags.
    pub fn set_style<S>(&self, style: S) -> io::Result<()>
            where S: Into<Option<Style>> {
        self.0.set_style(style.into().unwrap_or_default())
    }

    /// Sets all attributes for the terminal.
    pub fn set_theme(&self, theme: Theme) -> io::Result<()> {
        self.0.set_theme(theme)
    }

    /// Sets the foreground text color.
    pub fn set_fg<C: Into<Option<Color>>>(&self, fg: C) -> io::Result<()> {
        self.0.set_fg(fg.into())
    }

    /// Sets the background text color.
    pub fn set_bg<C: Into<Option<Color>>>(&self, bg: C) -> io::Result<()> {
        self.0.set_bg(bg.into())
    }

    /// Removes color and style attributes.
    pub fn clear_attributes(&self) -> io::Result<()> {
        self.0.clear_attributes()
    }

    /// Adds bold to the current style setting.
    ///
    /// This is equivalent to `self.add_style(Style::BOLD)`.
    pub fn bold(&self) -> io::Result<()> {
        self.add_style(Style::BOLD)
    }

    /// Adds italic to the current style setting.
    ///
    /// This is equivalent to `self.add_style(Style::ITALIC)`.
    pub fn italic(&self) -> io::Result<()> {
        self.add_style(Style::ITALIC)
    }

    /// Adds underline to the current style setting.
    ///
    /// This is equivalent to `self.add_style(Style::UNDERLINE)`.
    pub fn underline(&self) -> io::Result<()> {
        self.add_style(Style::UNDERLINE)
    }

    /// Adds reverse to the current style setting.
    ///
    /// This is equivalent to `self.add_style(Style::REVERSE)`.
    pub fn reverse(&self) -> io::Result<()> {
        self.add_style(Style::REVERSE)
    }

    /// Writes output to the terminal with the given color and style.
    ///
    /// All attributes are removed after the given text is written.
    pub fn write_styled<F, B, S>(&self, fg: F, bg: B, style: S, s: &str)
            -> io::Result<()> where
            F: Into<Option<Color>>,
            B: Into<Option<Color>>,
            S: Into<Option<Style>>,
            {
        self.0.write_styled(fg.into(), bg.into(), style.into().unwrap_or_default(), s)
    }

    /// Writes a single character to the terminal
    /// using the current style and color settings.
    pub fn write_char(&self, ch: char) -> io::Result<()> {
        self.0.write_char(ch)
    }

    /// Writes a string to the terminal
    /// using the current style and color settings.
    pub fn write_str(&self, s: &str) -> io::Result<()> {
        self.0.write_str(s)
    }

    /// Writes formatted text to the terminal
    /// using the current style and color settings.
    ///
    /// This method enables `Terminal` to be used as the receiver to
    /// the [`write!`] and [`writeln!`] macros.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use std::io;
    /// # use mortal::Terminal;
    /// # fn example() -> io::Result<()> {
    /// let term = Terminal::new()?;
    ///
    /// writeln!(term, "Hello, world!")?;
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// [`write!`]: https://doc.rust-lang.org/std/macro.write.html
    /// [`writeln!`]: https://doc.rust-lang.org/std/macro.writeln.html
    pub fn write_fmt(&self, args: fmt::Arguments) -> io::Result<()> {
        let s = args.to_string();
        self.write_str(&s)
    }

    #[doc(hidden)]
    pub fn borrow_term_write_guard(&self) -> TerminalWriteGuard {
        self.lock_write().unwrap()
    }
}

impl<'a> TerminalReadGuard<'a> {
    /// Prepares the terminal to read input.
    ///
    /// When reading operations have concluded, [`restore`]
    /// should be called with the resulting `PrepareState` value to restore
    /// the terminal to its previous state.
    ///
    /// This method may be called more than once before a corresponding
    /// `restore` call. However, each `restore` call must receive the most recently
    /// created `PrepareState` value.
    ///
    /// See [`PrepareConfig`] for details.
    ///
    /// [`PrepareConfig`]: struct.PrepareConfig.html
    /// [`restore`]: #method.restore
    ///
    /// ## Locking
    ///
    /// This method internally acquires the [`Terminal`] write lock.
    ///
    /// If the write lock is already held by the current thread,
    /// call [`prepare_with_lock`], in order to prevent deadlocks.
    ///
    /// [`Terminal`]: struct.Terminal.html
    /// [`prepare_with_lock`]: #method.prepare_with_lock
    pub fn prepare(&mut self, config: PrepareConfig) -> io::Result<PrepareState> {
        self.0.prepare(config).map(PrepareState)
    }

    /// Performs terminal preparation using both [`Terminal`] locks.
    ///
    /// [`Terminal`]: struct.Terminal.html
    pub fn prepare_with_lock(&mut self, writer: &mut TerminalWriteGuard,
            config: PrepareConfig) -> io::Result<PrepareState> {
        self.0.prepare_with_lock(&mut writer.0, config).map(PrepareState)
    }

    /// Restores the terminal to its previous state.
    ///
    /// ## Locking
    ///
    /// This method internally acquires the [`Terminal`] write lock.
    ///
    /// If the write lock is already held by the current thread,
    /// call [`restore_with_lock`], in order to prevent deadlocks.
    ///
    /// [`Terminal`]: struct.Terminal.html
    /// [`restore_with_lock`]: #method.restore_with_lock
    pub fn restore(&mut self, state: PrepareState) -> io::Result<()> {
        self.0.restore(state.0)
    }

    /// Performs terminal state restoration using both [`Terminal`] locks.
    ///
    /// [`Terminal`]: struct.Terminal.html
    pub fn restore_with_lock(&mut self, writer: &mut TerminalWriteGuard,
            state: PrepareState) -> io::Result<()> {
        self.0.restore_with_lock(&mut writer.0, state.0)
    }

    /// Waits for an event from the terminal.
    ///
    /// Returns `Ok(false)` if `timeout` elapses without an event occurring.
    ///
    /// If `timeout` is `None`, this method will wait indefinitely.
    pub fn wait_event(&mut self, timeout: Option<Duration>) -> io::Result<bool> {
        self.0.wait_event(timeout)
    }

    /// Waits for input and reads an event from the terminal.
    ///
    /// Returns `Ok(None)` if `timeout` elapses without an event occurring.
    ///
    /// If `timeout` is `None`, this method will wait indefinitely.
    pub fn read_event(&mut self, timeout: Option<Duration>) -> io::Result<Option<Event>>  {
        self.0.read_event(timeout)
    }
}

impl<'a> TerminalWriteGuard<'a> {
    /// Flush all output to the terminal device.
    ///
    /// This is called automatically when the `TerminalWriteGuard` is dropped.
    pub fn flush(&mut self) -> io::Result<()> {
        self.0.flush()
    }

    /// Returns the size of the terminal.
    #[inline]
    pub fn size(&self) -> io::Result<Size> {
        self.0.size()
    }

    /// Clears the terminal screen, placing the cursor at the first line and column.
    ///
    /// If the terminal contains a scrolling window over a buffer, the window
    /// will be scrolled downward, preserving as much of the existing buffer
    /// as possible.
    pub fn clear_screen(&mut self) -> io::Result<()> {
        self.0.clear_screen()
    }

    /// Clears the current line, starting at cursor position.
    pub fn clear_to_line_end(&mut self) -> io::Result<()> {
        self.0.clear_to_line_end()
    }

    /// Clears the screen, starting at cursor position.
    pub fn clear_to_screen_end(&mut self) -> io::Result<()> {
        self.0.clear_to_screen_end()
    }

    /// Moves the cursor up `n` lines.
    pub fn move_up(&mut self, n: usize) -> io::Result<()> {
        self.0.move_up(n)
    }

    /// Moves the cursor down `n` lines.
    pub fn move_down(&mut self, n: usize) -> io::Result<()> {
        self.0.move_down(n)
    }

    /// Moves the cursor left `n` columns.
    pub fn move_left(&mut self, n: usize) -> io::Result<()> {
        self.0.move_left(n)
    }

    /// Moves the cursor right `n` columns.
    pub fn move_right(&mut self, n: usize) -> io::Result<()> {
        self.0.move_right(n)
    }

    /// Moves the cursor to the first column of the current line
    pub fn move_to_first_column(&mut self) -> io::Result<()> {
        self.0.move_to_first_column()
    }

    /// Set the current cursor mode.
    ///
    /// This setting is a visible hint to the user.
    /// It produces no change in behavior.
    ///
    /// # Notes
    ///
    /// On some systems, this setting may have no effect.
    pub fn set_cursor_mode(&mut self, mode: CursorMode) -> io::Result<()> {
        self.0.set_cursor_mode(mode)
    }

    /// Adds a set of `Style` flags to the current style setting.
    pub fn add_style(&mut self, style: Style) -> io::Result<()> {
        self.0.add_style(style)
    }

    /// Removes a set of `Style` flags from the current style setting.
    pub fn remove_style(&mut self, style: Style) -> io::Result<()> {
        self.0.remove_style(style)
    }

    /// Sets the current style to the given set of flags.
    pub fn set_style<S>(&mut self, style: S) -> io::Result<()>
            where S: Into<Option<Style>> {
        self.0.set_style(style.into().unwrap_or_default())
    }

    /// Sets all attributes for the terminal.
    pub fn set_theme(&mut self, theme: Theme) -> io::Result<()> {
        self.0.set_theme(theme)
    }

    /// Sets the background text color.
    pub fn set_fg<C: Into<Option<Color>>>(&mut self, fg: C) -> io::Result<()> {
        self.0.set_fg(fg.into())
    }

    /// Removes color and style attributes.
    pub fn set_bg<C: Into<Option<Color>>>(&mut self, bg: C) -> io::Result<()> {
        self.0.set_bg(bg.into())
    }

    /// Adds bold to the current style setting.
    pub fn clear_attributes(&mut self) -> io::Result<()> {
        self.0.clear_attributes()
    }

    /// Adds bold to the current style setting.
    ///
    /// This is equivalent to `self.add_style(Style::BOLD)`.
    pub fn bold(&mut self) -> io::Result<()> {
        self.add_style(Style::BOLD)
    }

    /// Adds italic to the current style setting.
    ///
    /// This is equivalent to `self.add_style(Style::ITALIC)`.
    pub fn italic(&mut self) -> io::Result<()> {
        self.add_style(Style::ITALIC)
    }

    /// Adds underline to the current style setting.
    ///
    /// This is equivalent to `self.add_style(Style::UNDERLINE)`.
    pub fn underline(&mut self) -> io::Result<()> {
        self.add_style(Style::UNDERLINE)
    }

    /// Adds reverse to the current style setting.
    ///
    /// This is equivalent to `self.add_style(Style::REVERSE)`.
    pub fn reverse(&mut self) -> io::Result<()> {
        self.add_style(Style::REVERSE)
    }

    /// Writes output to the terminal with the given color and style added.
    ///
    /// All attributes are removed after the given text is written.
    pub fn write_styled<F, B, S>(&mut self, fg: F, bg: B, style: S, s: &str)
            -> io::Result<()> where
            F: Into<Option<Color>>,
            B: Into<Option<Color>>,
            S: Into<Option<Style>>,
            {
        self.0.write_styled(fg.into(), bg.into(), style.into().unwrap_or_default(), s)
    }

    /// Writes a single character to the terminal
    /// using the current style and color settings.
    pub fn write_char(&mut self, ch: char) -> io::Result<()> {
        self.0.write_char(ch)
    }

    /// Writes a string to the terminal
    /// using the current style and color settings.
    pub fn write_str(&mut self, s: &str) -> io::Result<()> {
        self.0.write_str(s)
    }

    /// Writes formatted text to the terminal
    /// using the current style and color settings.
    ///
    /// This method enables `TerminalWriteGuard` to be used as the receiver to
    /// the [`write!`] and [`writeln!`] macros.
    ///
    /// [`write!`]: https://doc.rust-lang.org/std/macro.write.html
    /// [`writeln!`]: https://doc.rust-lang.org/std/macro.writeln.html
    pub fn write_fmt(&mut self, args: fmt::Arguments) -> io::Result<()> {
        let s = args.to_string();
        self.write_str(&s)
    }

    #[doc(hidden)]
    pub fn borrow_term_write_guard(&mut self) -> &mut Self {
        self
    }
}

#[cfg(unix)]
use std::path::Path;

#[cfg(unix)]
impl crate::unix::OpenTerminalExt for Terminal {
    fn from_path<P: AsRef<Path>>(path: P) -> io::Result<Self> {
        sys::Terminal::open(path).map(Terminal)
    }
}

#[cfg(unix)]
impl crate::unix::TerminalExt for Terminal {
    fn read_raw(&mut self, buf: &mut [u8], timeout: Option<Duration>) -> io::Result<Option<Event>> {
        self.0.read_raw(buf, timeout)
    }
}

#[cfg(unix)]
impl<'a> crate::unix::TerminalExt for TerminalReadGuard<'a> {
    fn read_raw(&mut self, buf: &mut [u8], timeout: Option<Duration>) -> io::Result<Option<Event>> {
        self.0.read_raw(buf, timeout)
    }
}

#[cfg(windows)]
impl crate::windows::TerminalExt for Terminal {
    fn read_raw(&mut self, buf: &mut [u16], timeout: Option<Duration>) -> io::Result<Option<Event>> {
        self.0.read_raw(buf, timeout)
    }

    fn read_raw_event(&mut self, events: &mut [::winapi::um::wincon::INPUT_RECORD],
            timeout: Option<Duration>) -> io::Result<Option<Event>> {
        self.0.read_raw_event(events, timeout)
    }
}

#[cfg(windows)]
impl<'a> crate::windows::TerminalExt for TerminalReadGuard<'a> {
    fn read_raw(&mut self, buf: &mut [u16], timeout: Option<Duration>) -> io::Result<Option<Event>> {
        self.0.read_raw(buf, timeout)
    }

    fn read_raw_event(&mut self, events: &mut [::winapi::um::wincon::INPUT_RECORD],
            timeout: Option<Duration>) -> io::Result<Option<Event>> {
        self.0.read_raw_event(events, timeout)
    }
}
