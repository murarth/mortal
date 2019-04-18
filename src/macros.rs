//! Provides macros easier printing with colors and styles.

use std::io;

/// Writes attributes and formatted text to a `Terminal` or `Screen`.
///
/// # Usage
///
/// `term_write!` accepts a series of attribute elements and formatted text elements.
///
/// [`term_writeln!`] is equivalent, but writes a newline character
/// to the end of the formatted text.
///
/// Attribute elements are enclosed in square brackets
/// and take one of the following forms:
///
/// | Element           | Equivalent                        |
/// | ----------------- | --------------------------------- |
/// | `[red]`           | `term.set_fg(Color::Red)`         |
/// | `[#blue]`         | `term.set_bg(Color::Blue)`        |
/// | `[bold]`          | `term.add_style(Style::BOLD)`     |
/// | `[!bold]`         | `term.remove_style(Style::BOLD)`  |
/// | `[reset]`         | `term.clear_attributes()`         |
/// | `[!fg]`           | `term.set_fg(None)`               |
/// | `[!bg]`           | `term.set_bg(None)`               |
/// | `[!style]`        | `term.set_style(None)`            |
/// | `[fg=expr]`       | `term.set_fg(expr)`               |
/// | `[bg=expr]`       | `term.set_bg(expr)`               |
/// | `[style=expr]`    | `term.set_style(expr)`            |
/// | `[style+=expr]`   | `term.add_style(expr)`            |
/// | `[style-=expr]`   | `term.remove_style(expr)`         |
/// | `[theme=expr]`    | `term.set_theme(expr)`            |
///
/// Formatted text elements are enclosed in parentheses
/// and use Rust [`std::fmt`] functions to write formatted text to the terminal.
/// Additionally, a bare string literal may be given and will be written
/// directly to the terminal.
///
/// | Element           | Equivalent                        |
/// | ----------------- | --------------------------------- |
/// | `(: expr)`        | `write!(term, "{}", expr)`        |
/// | `(? expr)`        | `write!(term, "{:?}", expr)`      |
/// | `("format", ...)` | `write!(term, "format", ...)`     |
/// | `"literal str"`   | `term.write_str("literal str")`   |
///
/// # Examples
///
/// ```no_run
/// #[macro_use] extern crate mortal;
/// # use std::io;
/// use mortal::{Color, Style, Theme, Terminal};
///
/// # fn main() -> io::Result<()> {
/// let term = Terminal::new()?;
///
/// term_writeln!(term, [red] "red text" [reset])?;
///
/// let color = Color::Green;
/// term_writeln!(term, [fg=color] "green text" [reset])?;
///
/// let style = Style::BOLD;
/// term_writeln!(term, [style=style] "bold text" [reset])?;
///
/// let value = 42;
/// term_writeln!(term, "The answer is: " [bold] (: value) [reset])?;
///
/// let theme = Theme::new(color, None, style);
/// term_writeln!(term, [theme=theme] "Green, bold text" [reset])?;
/// # Ok(())
/// # }
/// ```
///
/// [`std::fmt`]: https://doc.rust-lang.org/std/fmt/
/// [`term_writeln!`]: macro.term_writeln.html
#[macro_export]
macro_rules! term_write {
    // Entry rule
    ( $term:expr , $first:tt $($rest:tt)* ) => {
        match $term.borrow_term_write_guard() {
            mut term => {
                let init = $crate::macros::Chain::init();
                term_write!(@_INTERNAL main: term ; init ; $first $($rest)*)
            }
        }
    };

    // Final rule
    ( @_INTERNAL main: $term:expr ; $result:expr ; ) => {
        $result
    };

    // Color/style rules
    ( @_INTERNAL main: $term:expr ; $result:expr ; [ $($tt:tt)* ] $($rest:tt)* ) => {
        term_write!(
            @_INTERNAL main: $term;
            term_write!(@_INTERNAL style: $term; $result; $($tt)*);
            $($rest)*
        )
    };

    // Formatting rules
    ( @_INTERNAL main: $term:expr ; $result:expr ; ( $($tt:tt)* ) $($rest:tt)* ) => {
        term_write!(
            @_INTERNAL main: $term;
            term_write!(@_INTERNAL format: $term; $result; $($tt)*);
            $($rest)*
        )
    };
    ( @_INTERNAL main: $term:expr ; $result:expr ; $tt:tt $($rest:tt)* ) => {
        term_write!(
            @_INTERNAL main: $term;
            term_write!(@_INTERNAL literal: $term; $result; $tt);
            $($rest)*
        )
    };

    // Set foreground color
    ( @_INTERNAL style: $term:expr ; $result:expr ; black ) => {
        $crate::macros::Chain::chain(
            $result, || $term.set_fg($crate::Color::Black))
    };
    ( @_INTERNAL style: $term:expr ; $result:expr ; blue ) => {
        $crate::macros::Chain::chain(
            $result, || $term.set_fg($crate::Color::Blue))
    };
    ( @_INTERNAL style: $term:expr ; $result:expr ; cyan ) => {
        $crate::macros::Chain::chain(
            $result, || $term.set_fg($crate::Color::Cyan))
    };
    ( @_INTERNAL style: $term:expr ; $result:expr ; green ) => {
        $crate::macros::Chain::chain(
            $result, || $term.set_fg($crate::Color::Green))
    };
    ( @_INTERNAL style: $term:expr ; $result:expr ; magenta ) => {
        $crate::macros::Chain::chain(
            $result, || $term.set_fg($crate::Color::Magenta))
    };
    ( @_INTERNAL style: $term:expr ; $result:expr ; red ) => {
        $crate::macros::Chain::chain(
            $result, || $term.set_fg($crate::Color::Red))
    };
    ( @_INTERNAL style: $term:expr ; $result:expr ; white ) => {
        $crate::macros::Chain::chain(
            $result, || $term.set_fg($crate::Color::White))
    };
    ( @_INTERNAL style: $term:expr ; $result:expr ; yellow ) => {
        $crate::macros::Chain::chain(
            $result, || $term.set_fg($crate::Color::Yellow))
    };

    // Set background color
    ( @_INTERNAL style: $term:expr ; $result:expr ; # black ) => {
        $crate::macros::Chain::chain(
            $result, || $term.set_bg($crate::Color::Black))
    };
    ( @_INTERNAL style: $term:expr ; $result:expr ; # blue ) => {
        $crate::macros::Chain::chain(
            $result, || $term.set_bg($crate::Color::Blue))
    };
    ( @_INTERNAL style: $term:expr ; $result:expr ; # cyan ) => {
        $crate::macros::Chain::chain(
            $result, || $term.set_bg($crate::Color::Cyan))
    };
    ( @_INTERNAL style: $term:expr ; $result:expr ; # green ) => {
        $crate::macros::Chain::chain(
            $result, || $term.set_bg($crate::Color::Green))
    };
    ( @_INTERNAL style: $term:expr ; $result:expr ; # magenta ) => {
        $crate::macros::Chain::chain(
            $result, || $term.set_bg($crate::Color::Magenta))
    };
    ( @_INTERNAL style: $term:expr ; $result:expr ; # red ) => {
        $crate::macros::Chain::chain(
            $result, || $term.set_bg($crate::Color::Red))
    };
    ( @_INTERNAL style: $term:expr ; $result:expr ; # white ) => {
        $crate::macros::Chain::chain(
            $result, || $term.set_bg($crate::Color::White))
    };
    ( @_INTERNAL style: $term:expr ; $result:expr ; # yellow ) => {
        $crate::macros::Chain::chain(
            $result, || $term.set_bg($crate::Color::Yellow))
    };

    // Add style
    ( @_INTERNAL style: $term:expr ; $result:expr ; bold ) => {
        $crate::macros::Chain::chain(
            $result, || $term.add_style($crate::Style::BOLD))
    };
    ( @_INTERNAL style: $term:expr ; $result:expr ; italic ) => {
        $crate::macros::Chain::chain(
            $result, || $term.add_style($crate::Style::ITALIC))
    };
    ( @_INTERNAL style: $term:expr ; $result:expr ; reverse ) => {
        $crate::macros::Chain::chain(
            $result, || $term.add_style($crate::Style::REVERSE))
    };
    ( @_INTERNAL style: $term:expr ; $result:expr ; underline ) => {
        $crate::macros::Chain::chain(
            $result, || $term.add_style($crate::Style::UNDERLINE))
    };

    // Remove style
    ( @_INTERNAL style: $term:expr ; $result:expr ; ! bold ) => {
        $crate::macros::Chain::chain(
            $result, || $term.remove_style($crate::Style::BOLD))
    };
    ( @_INTERNAL style: $term:expr ; $result:expr ; ! italic ) => {
        $crate::macros::Chain::chain(
            $result, || $term.remove_style($crate::Style::ITALIC))
    };
    ( @_INTERNAL style: $term:expr ; $result:expr ; ! reverse ) => {
        $crate::macros::Chain::chain(
            $result, || $term.remove_style($crate::Style::REVERSE))
    };
    ( @_INTERNAL style: $term:expr ; $result:expr ; ! underline ) => {
        $crate::macros::Chain::chain(
            $result, || $term.remove_style($crate::Style::UNDERLINE))
    };

    // Clear attributes
    ( @_INTERNAL style: $term:expr ; $result:expr ; reset ) => {
        $crate::macros::Chain::chain(
            $result, || $term.clear_attributes())
    };
    ( @_INTERNAL style: $term:expr ; $result:expr ; ! fg ) => {
        $crate::macros::Chain::chain(
            $result, || $term.set_fg(None))
    };
    ( @_INTERNAL style: $term:expr ; $result:expr ; ! bg ) => {
        $crate::macros::Chain::chain(
            $result, || $term.set_bg(None))
    };
    ( @_INTERNAL style: $term:expr ; $result:expr ; ! style ) => {
        $crate::macros::Chain::chain(
            $result, || $term.set_style(None))
    };

    // Color/style expressions
    ( @_INTERNAL style: $term:expr ; $result:expr ; fg = $e:expr ) => {
        $crate::macros::Chain::chain(
            $result, || $term.set_fg($e))
    };
    ( @_INTERNAL style: $term:expr ; $result:expr ; bg = $e:expr ) => {
        $crate::macros::Chain::chain(
            $result, || $term.set_bg($e))
    };
    ( @_INTERNAL style: $term:expr ; $result:expr ; style = $e:expr ) => {
        $crate::macros::Chain::chain(
            $result, || $term.set_style($e))
    };
    ( @_INTERNAL style: $term:expr ; $result:expr ; style += $e:expr ) => {
        $crate::macros::Chain::chain(
            $result, || $term.add_style($e))
    };
    ( @_INTERNAL style: $term:expr ; $result:expr ; style -= $e:expr ) => {
        $crate::macros::Chain::chain(
            $result, || $term.remove_style($e))
    };
    ( @_INTERNAL style: $term:expr ; $result:expr ; theme = $e:expr ) => {
        $crate::macros::Chain::chain(
            $result, || $term.set_theme($e))
    };

    // std::fmt formatting
    ( @_INTERNAL format: $term:expr ; $result:expr ; : $e:expr ) => {
        $crate::macros::Chain::chain(
            $result, || write!($term, "{}", $e))
    };
    ( @_INTERNAL format: $term:expr ; $result:expr ; ? $e:expr ) => {
        $crate::macros::Chain::chain(
            $result, || write!($term, "{:?}", $e))
    };
    ( @_INTERNAL format: $term:expr ; $result:expr ; $($tt:tt)* ) => {
        $crate::macros::Chain::chain(
            $result, || write!($term, $($tt)*))
    };

    // Literal formatting
    ( @_INTERNAL literal: $term:expr ; $result:expr ; $lit:tt ) => {
        $crate::macros::Chain::chain(
            $result, || $term.write_str(concat!($lit)))
    };
}

/// Writes attributes and formatted text to a `Terminal` or `Screen`.
///
/// Formatted output is followed by a newline.
///
/// See [`term_write`] for a description of macro syntax and example usage.
///
/// [`term_write`]: macro.term_write.html
#[macro_export]
macro_rules! term_writeln {
    ( $term:expr ) => {
        term_write!($term, "\n")
    };
    ( $term:expr , $($tt:tt)* ) => {
        term_write!($term, $($tt)* "\n")
    };
}

// Facilitates chaining calls from either a `Terminal` or `Screen` lock.
//
// Terminal methods return `io::Result<()>` and are chained with
// `Result::and_then`; Screen methods return `()`, so the next function is
// always called.
#[doc(hidden)]
pub trait Chain: Sized {
    fn chain<F: FnOnce() -> Self>(self, f: F) -> Self;

    fn init() -> Self;
}

impl Chain for () {
    fn chain<F: FnOnce() -> Self>(self, f: F) -> Self {
        f()
    }

    fn init() -> Self { }
}

impl Chain for io::Result<()> {
    fn chain<F: FnOnce() -> Self>(self, f: F) -> Self {
        self.and_then(|_| f())
    }

    fn init() -> Self { Ok(()) }
}
