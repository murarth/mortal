//! Example of reading raw input from the terminal

extern crate mortal;

use std::io;

use mortal::{Event, Terminal};

#[cfg(unix)]
use mortal::unix::TerminalExt;

#[cfg(windows)]
use mortal::windows::TerminalExt;

#[cfg(unix)]
type RawUnit = u8;

#[cfg(windows)]
type RawUnit = u16;

fn main() -> io::Result<()> {
    let mut term = Terminal::new()?;

    // Prepare to read from the terminal.
    let state = term.prepare(Default::default())?;

    println!("Reading input. Press 'q' to stop.");

    let mut buf = [0; 32];

    // Read raw input data from the terminal in native encoding.
    loop {
        if let Some(ev) = term.read_raw(&mut buf, None)? {

            match ev {
                Event::Raw(n) => {
                    println!("read {}: {:?}", n, &buf[..n]);

                    if n == 1 && buf[0] == b'q' as RawUnit {
                        break;
                    }
                }
                ev => {
                    println!("read event: {:?}", ev);
                }
            }
        }
    }

    // Restore terminal settings.
    term.restore(state)?;

    Ok(())
}
