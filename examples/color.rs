//! Example usage of color/styled terminal output

extern crate mortal;

use std::io;

use mortal::{Color, Style, Terminal};

fn main() -> io::Result<()> {
    let term = Terminal::new()?;

    // There are two ways to write color/styled text to the terminal.

    // 1. Set style/color methods and write text.
    term.bold()?;
    term.set_fg(Color::Red)?;
    write!(term, "error")?;

    // Remember to clear attributes when you want to write plain text again.
    term.clear_attributes()?;
    writeln!(term, ": error message")?;

    // 2. Use the `write_styled` method to write color/styled text.
    term.write_styled(Color::Green, None, Style::BOLD, "help")?;
    // After `write`, all attributes are cleared and text is plain.
    writeln!(term, ": help message")?;

    Ok(())
}
