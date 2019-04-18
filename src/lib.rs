//! Platform-independent terminal interface
//!
//! Two distinct interfaces to operating system terminal devices are provided,
//! each abstracting over the differences between Unix terminals and Windows console.
//!
//! The [`Terminal`] interface treats the terminal as a line-by-line
//! output device. Methods exist to add color and style attributes to text,
//! and to make relative movements of the cursor.
//!
//! The [`Screen`] interface treats the entire terminal window as a drawable
//! buffer. Methods exist to set the cursor position and to write text with
//! color and style attributes.
//!
//! The [`term_write!`] and [`term_writeln!`] macros provide a convenient interface
//! to output attributes and formatted text to either a `Terminal` or `Screen`
//! instance.
//!
//! ## Concurrency
//!
//! Each interface uses internal locking mechanisms to allow sharing of the
//! terminal interface between threads while maintaining coherence of read/write
//! operations.
//!
//! See the documentation for [`Terminal`] and [`Screen`] for further details.
//!
//! [`Screen`]: screen/struct.Screen.html
//! [`Terminal`]: terminal/struct.Terminal.html
//! [`term_write!`]: macro.term_write.html
//! [`term_writeln!`]: macro.term_writeln.html

#![deny(missing_docs)]

#[macro_use] extern crate bitflags;
extern crate smallstr;
extern crate unicode_normalization;
extern crate unicode_width;

#[cfg(unix)] extern crate libc;
#[cfg(unix)] extern crate nix;
#[cfg(unix)] extern crate terminfo;

#[cfg(windows)] extern crate winapi;

pub use crate::screen::{Screen, ScreenReadGuard, ScreenWriteGuard};
pub use crate::sequence::{FindResult, SequenceMap};
pub use crate::signal::{Signal, SignalSet};
pub use crate::terminal::{
    Color, Cursor, CursorMode, Size, Style, Theme,
    Event, Key, MouseEvent, MouseInput, MouseButton, ModifierState,
    PrepareConfig, PrepareState,
    Terminal, TerminalReadGuard, TerminalWriteGuard,
};

#[macro_use] mod buffer;
#[doc(hidden)]
#[macro_use] pub mod macros;
mod priv_util;
pub mod screen;
pub mod sequence;
pub mod signal;
pub mod terminal;
pub mod util;

#[cfg(unix)]
#[path = "unix/mod.rs"]
mod sys;

#[cfg(windows)]
#[path = "windows/mod.rs"]
mod sys;

#[cfg(unix)]
pub use crate::sys::ext as unix;

#[cfg(windows)]
pub use sys::ext as windows;

#[cfg(test)]
mod test {
    use crate::screen::Screen;
    use crate::terminal::Terminal;

    fn assert_has_traits<T: 'static + Send + Sync>() {}

    #[test]
    fn test_traits() {
        assert_has_traits::<Terminal>();
        assert_has_traits::<Screen>();
    }
}
