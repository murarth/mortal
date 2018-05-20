//! Unix extension trait

use std::io;
use std::time::Duration;

use priv_util::Private;
use terminal::Event;

/// Implements extensions for `Terminal` and `Screen` on Unix systems.
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
