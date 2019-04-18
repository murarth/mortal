//! Unix extension trait

use std::io;
use std::path::Path;
use std::time::Duration;

use crate::priv_util::Private;
use crate::terminal::Event;

/// Implements Unix-only extensions for terminal interfaces.
pub trait OpenTerminalExt: Sized + Private {
    /// Opens a terminal interface on the device at the given path.
    ///
    /// If the path cannot be opened for read/write operations,
    /// an error is returned.
    fn from_path<P: AsRef<Path>>(path: P) -> io::Result<Self>;
}

/// Implements Unix-only extensions for terminal interfaces.
pub trait TerminalExt: Private {
    /// Reads raw data from the terminal.
    ///
    /// Data read will be UTF-8 encoded, but may be incomplete. The caller may
    /// consume any valid UTF-8 data before performing another `read_raw` call
    /// to complete previously read data.
    ///
    /// If `timeout` elapses without an event occurring, this method will return
    /// `Ok(None)`. If `timeout` is `None`, this method will wait indefinitely.
    fn read_raw(&mut self, buf: &mut [u8], timeout: Option<Duration>)
            -> io::Result<Option<Event>>;
}
