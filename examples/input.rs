//! Example of reading input events from the terminal

extern crate mortal;

use std::io;

use mortal::{Event, Key, PrepareConfig, Terminal};

fn main() -> io::Result<()> {
    let term = Terminal::new()?;

    // Prepare to read from the terminal.
    let state = term.prepare(PrepareConfig{
        enable_mouse: true,
        .. Default::default()
    })?;

    println!("Reading input. Press 'q' to stop.");

    // Read input from the terminal, one key at a time.
    loop {
        if let Some(ev) = term.read_event(None)? {
            if let Event::NoEvent = ev {
                continue;
            }

            println!("read event: {:?}", ev);

            if let Event::Key(Key::Char('q')) = ev {
                break;
            }
        }
    }

    // Restore terminal settings.
    term.restore(state)?;

    Ok(())
}
