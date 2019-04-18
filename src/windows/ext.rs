//! Windows console extension trait

use std::io;
use std::time::Duration;

use winapi::um::wincon::INPUT_RECORD;

use crate::priv_util::Private;
use crate::terminal::Event;

/// Implements Windows-only extensions for terminal interfaces.
pub trait TerminalExt: Private {
    /// Reads raw data from the console.
    ///
    /// Data read will be UTF-16 encoded, but may be incomplete. The caller may
    /// consume any valid UTF-16 data before performing another `read_raw` call
    /// to complete previously read data.
    ///
    /// If `timeout` elapses without an event occurring, this method will return
    /// `Ok(None)`. If `timeout` is `None`, this method will wait indefinitely.
    fn read_raw(&mut self, buf: &mut [u16], timeout: Option<Duration>)
            -> io::Result<Option<Event>>;

    /// Reads raw event data from the console.
    ///
    /// If `timeout` elapses without an event occurring, this method will return
    /// `Ok(None)`. If `timeout` is `None`, this method will wait indefinitely.
    fn read_raw_event(&mut self, events: &mut [INPUT_RECORD],
            timeout: Option<Duration>) -> io::Result<Option<Event>>;
}
