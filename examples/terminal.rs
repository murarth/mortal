//! Example using cursor movement in Terminal mode

extern crate mortal;

use std::io;

use mortal::{Event, Key, Terminal};

fn main() -> io::Result<()> {
    let mut term = Terminal::new()?;

    let mut n_events = 0;
    let mut last_event = None;

    // Prepare to read from the terminal.
    let state = term.prepare(Default::default())?;

    println!("Collecting input. Press 'q' to stop.");
    println!();

    // Write a few empty lines for where our text will be.
    println!();
    println!();
    println!();

    write_data(&mut term, n_events, last_event)?;

    loop {
        if let Some(ev) = term.read_event(None)? {
            if let Event::NoEvent = ev {
                continue;
            }

            n_events += 1;
            last_event = Some(ev);

            write_data(&mut term, n_events, last_event)?;

            if let Event::Key(Key::Char('q')) = ev {
                break;
            }
        }
    }

    // Restore terminal state
    term.restore(state)?;

    Ok(())
}

fn write_data(term: &mut Terminal, n_events: usize, last_event: Option<Event>)
        -> io::Result<()> {
    // Move the cursor up 2 lines.
    term.move_up(2)?;
    // Clear all text from the previous write.
    term.clear_to_screen_end()?;

    writeln!(term, "Number of events: {}", n_events)?;

    if let Some(ev) = last_event {
        writeln!(term, "Last event: {:?}", ev)?;
    } else {
        writeln!(term, "Last event: None")?;
    }

    Ok(())
}
