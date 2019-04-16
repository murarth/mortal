//! Provides macros easier printing with colors and styles.
//!
//! See: https://github.com/murarth/mortal/issues/7




/// Writes on given terminal using themes.
///
/// # Syntax
///
/// The base syntax of this macro is:
///
/// ```ignore
/// term_write!( [lock] <terminal> ; [theming|output] ... )
/// ```
///
/// First there is the optional keyword `lock` followed by the identifier
/// (aka name) of the terminal variable. This variable must be of type
/// [`Terminal`] or [`TerminalWriteGuard`]. If the type of that
/// variable is `Terminal` the keyword `lock` may be specified too.
/// After the terminal variable follow, separated by a semicolon `;`, either
/// _theme specifications_ or _output instructions_, or multiple of them.
///
/// The _theme specifications_ are fenced by square bracket `[ ]`. Within these
///  is either of the following:
/// - a foreground color: small caps color name such as (`black`, `blue`,
///   `cyan`, `green`, `magenta`, `red`, `white`, `yellow`)
/// - a background color: similar to foreground but with a leading hash `#` such
///   as (`#black`, `#blue`, `#cyan`, `#green`, `#magenta`, `#red`, `#white`,
///   `#yellow`)
/// - an additive style specifier: a small caps style name such as (`bold`,
///   `italic`, `underline`, `reverse`)
/// - a subtractive style specifier: similar to additive but with a
///   leading exclamation mark such as (`!bold`, `!italic`, `!underline`,
///   `!reverse`)
/// - a reset specifier either of `reset`, `!fg`, `!bg`, `!sty`
/// - foreground variable: `fg=` plus the name of a variable
/// - background variable: `bg=` plus the name of a variable
/// - style variable: `sty=` plus the name of a variable
/// - theme variable: `=` plus the name of a variable
///
/// The _output instructions_ may be either of the following:
/// - a literal such as a string `"stuff"` or integer `42`
/// - a Rust `std::fmt` format string with arguments fenced in
///   parentheses `( )` such as `("some {}", "text")`
/// - a `Display` shortcut for the format string `"{}"`, which is an expression
///   enclosed in parentheses with a leading colon `(: )` such as `(: true)`
/// - a `Debug` shortcut for the format string `"{:?}"`, which is an expression
///   enclosed in parentheses with a leading question mark `(? )` such as
///   `(? true)`. 
///
/// [`Terminal`]: ./terminal/struct.Terminal.html
/// [`TerminalWriteGuard`]: terminal/struct.TerminalWriteGuard.html
///
///
/// # Locking
///
/// The terminal write lock will be acquired as required. If the first argument
/// is of type `TerminalWriteGuard` no locking will be performed since the guard
/// already holds the lock. If the first argument is of type `Terminal` then
/// the keyword `lock` may be prefixed. If `lock` is given, then the macro will
/// acquire the terminal guard before any output and write out all output while
/// holding it. If the `lock` prefix is not given, each output fragment will lock
/// the terminal independently, which might lead to actual
/// fragmented output if there is concurrent thread writing to the terminal.
///
/// Notice that if the `TerminalWriteGuard` is held by the current thread,
/// then this macro must not be called with the `Terminal`. Otherwise a deadlock
/// will occur.
///
///
/// # Examples
///
/// ```
/// #[macro_use] extern crate mortal;
/// use mortal::Terminal;
/// use mortal::Theme;
/// use mortal::Color;
/// use mortal::Style;
///
/// let term = Terminal::new().unwrap();
/// // Simple output example
/// term_write!(term; "Hello world");
///
/// // Writing format strings
/// term_write!(term; ("Number #{}", 42));
///
/// // Using keywords
/// term_write!(term; [blue] "A blue " [bold] "world");
///
/// // Using variables
/// let c = Color::Green;
/// term_write!(term; "Just " [fg=c] (? c)); // short cut for ("{:?}", c)
///
/// // Using themes
/// let theme = Theme::default().fg(Color::Red).style(Style::BOLD);
/// term_write!(term; [=theme] "Red and Bold");
/// ```
///
/// Further examples can be found in the examples folder of Mortal.
///
#[macro_export]
macro_rules! term_write {
	// Lock prefix
	( lock $term:ident; $($rest:tt)* ) => {
		// Scoped in order to force unlock at the end.
		// Otherwise, calling this method twice in row could deadlock.
		{
			let mut lock = $term.lock_write().unwrap();
			term_write!(lock; $( $rest )* );
		}
	};
	
	// Final rule
	( $term:ident $( ; )* ) => {
		$term.clear_attributes();
	};
	
	// Optional , and ; as separators
	( $term:ident; , $($rest:tt)* ) => {
		term_write!($term; $($rest)*);
	};
	( $term:ident; ; $($rest:tt)* ) => {
		term_write!($term; $($rest)*);
	};
	
	// Foreground Colors
	( $term:ident; [ black ] $($rest:tt)*) => {
		$term.set_fg($crate::Color::Black);
		term_write!($term; $($rest)*);
	};
	( $term:ident; [ blue ] $($rest:tt)*) => {
		$term.set_fg($crate::Color::Blue);
		term_write!($term; $($rest)*);
	};
	( $term:ident; [ cyan ] $($rest:tt)*) => {
		$term.set_fg($crate::Color::Cyan);
		term_write!($term; $($rest)*);
	};
	( $term:ident; [ green ] $($rest:tt)*) => {
		$term.set_fg($crate::Color::Green);
		term_write!($term; $($rest)*);
	};
	( $term:ident; [ magenta ] $($rest:tt)*) => {
		$term.set_fg($crate::Color::Magenta);
		term_write!($term; $($rest)*);
	};
	( $term:ident; [ red ] $($rest:tt)*) => {
		$term.set_fg($crate::Color::Red);
		term_write!($term; $($rest)*);
	};
	( $term:ident; [ white ] $($rest:tt)*) => {
		$term.set_fg($crate::Color::White);
		term_write!($term; $($rest)*);
	};
	( $term:ident; [ yellow ] $($rest:tt)*) => {
		$term.set_fg($crate::Color::Yellow);
		term_write!($term; $($rest)*);
	};
	
	// Background Colors
	( $term:ident; [ # black ] $($rest:tt)*) => {
		$term.set_bg($crate::Color::Black);
		term_write!($term; $($rest)*);
	};
	( $term:ident; [ # blue ] $($rest:tt)*) => {
		$term.set_bg($crate::Color::Blue);
		term_write!($term; $($rest)*);
	};
	( $term:ident; [ # cyan ] $($rest:tt)*) => {
		$term.set_bg($crate::Color::Cyan);
		term_write!($term; $($rest)*);
	};
	( $term:ident; [ # green ] $($rest:tt)*) => {
		$term.set_bg($crate::Color::Green);
		term_write!($term; $($rest)*);
	};
	( $term:ident; [ # magenta ] $($rest:tt)*) => {
		$term.set_bg($crate::Color::Magenta);
		term_write!($term; $($rest)*);
	};
	( $term:ident; [ # red ] $($rest:tt)*) => {
		$term.set_bg($crate::Color::Red);
		term_write!($term; $($rest)*);
	};
	( $term:ident; [ # white ] $($rest:tt)*) => {
		$term.set_bg($crate::Color::White);
		term_write!($term; $($rest)*);
	};
	( $term:ident; [ # yellow ] $($rest:tt)*) => {
		$term.set_bg($crate::Color::Yellow);
		term_write!($term; $($rest)*);
	};
	
	// Adding Style
	( $term:ident; [ bold ] $($rest:tt)*) => {
		$term.add_style($crate::Style::BOLD);
		term_write!($term; $($rest)*);
	};
	( $term:ident; [ italic ] $($rest:tt)*) => {
		$term.add_style($crate::Style::ITALIC);
		term_write!($term; $($rest)*);
	};
	( $term:ident; [ reverse ] $($rest:tt)*) => {
		$term.add_style($crate::Style::REVERSE);
		term_write!($term; $($rest)*);
	};
	( $term:ident; [ underline ] $($rest:tt)*) => {
		$term.add_style($crate::Style::UNDERLINE);
		term_write!($term; $($rest)*);
	};
	
	// Removing Style
	( $term:ident; [ ! bold ] $($rest:tt)*) => {
		$term.remove_style($crate::Style::BOLD);
		term_write!($term; $($rest)*);
	};
	( $term:ident; [ ! italic ] $($rest:tt)*) => {
		$term.remove_style($crate::Style::ITALIC);
		term_write!($term; $($rest)*);
	};
	( $term:ident; [ ! reverse ] $($rest:tt)*) => {
		$term.remove_style($crate::Style::REVERSE);
		term_write!($term; $($rest)*);
	};
	( $term:ident; [ ! underline ] $($rest:tt)*) => {
		$term.remove_style($crate::Style::UNDERLINE);
		term_write!($term; $($rest)*);
	};
	
	// Resets
	( $term:ident; [ reset ] $($rest:tt)*) => {
		$term.clear_attributes();
		term_write!($term; $($rest)*);
	};
	( $term:ident; [ ! fg ] $($rest:tt)*) => {
		$term.set_fg(None);
		term_write!($term; $($rest)*);
	};
	( $term:ident; [ ! bg ] $($rest:tt)*) => {
		$term.set_bg(None);
		term_write!($term; $($rest)*);
	};
	( $term:ident; [ ! sty ] $($rest:tt)*) => {
		$term.set_style($crate::Style::default());
		term_write!($term; $($rest)*);
	};
	
	// Complex formats
	( $term:ident; [ fg = $e:expr ] $($rest:tt)*) => {
		$term.set_fg($e);
		term_write!($term; $($rest)*);
	};
	( $term:ident; [ bg = $e:expr ] $($rest:tt)*) => {
		$term.set_fg($e);
		term_write!($term; $($rest)*);
	};
	( $term:ident; [ sty = $e:expr ] $($rest:tt)*) => {
		$term.set_style($e);
		term_write!($term; $($rest)*);
	};
	( $term:ident; [ sty += $e:expr ] $($rest:tt)*) => {
		$term.add_style($e);
		term_write!($term; $($rest)*);
	};
	( $term:ident; [ sty -= $e:expr ] $($rest:tt)*) => {
		$term.remove_style($e);
		term_write!($term; $($rest)*);
	};
	( $term:ident; [ = $var:expr ] $($rest:tt)*) => {
		let x = &$var;
		let th = x as &$crate::Theme;
		$term.set_fg(th.fg).unwrap();
		$term.set_bg(th.bg).unwrap();
		$term.set_style(th.style).unwrap();
		term_write!($term; $($rest)*);
	};
	
	// Single expressing printing
	( $term:ident; (: $e:expr ) $($rest:tt)*) => {
		write!($term, "{}", $e ).unwrap();
		term_write!($term; $($rest)*);
	};
	( $term:ident; (? $e:expr ) $($rest:tt)*) => {
		write!($term, "{:?}", $e ).unwrap();
		term_write!($term; $($rest)*);
	};
	
	// Format printing
	( $term:ident; ( $( $fmt:tt )+ ) $($rest:tt)*) => {
		write!($term, $( $fmt )+ ).unwrap();
		term_write!($term; $($rest)*);
	};
	
	// Literal printing
	( $term:ident; $s:tt $($rest:tt)*) => {
		write!($term, "{}", concat!($s)).unwrap();
		term_write!($term; $($rest)*);
	};
}


/// Same as term_write macro but adds a new line at the end.
#[macro_export]
macro_rules! term_writeln {
	( lock $term:ident; $( $rest:tt )* ) => {
		// Scoped in order to force unlock at the end.
		// Otherwise, calling this method twice in row could deadlock.
		{
			let mut lock = $term.lock_write().unwrap();
			term_write!(lock; $( $rest )* );
			writeln!(lock).unwrap();
		}
	};
	( $term:ident $( ; )* ) => {
		writeln!($term).unwrap();
	};
	( $term:ident; $( $rest:tt )* ) => {
		term_write!($term; $( $rest )* );
		writeln!($term).unwrap();
	};
}


