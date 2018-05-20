extern crate mortal;

use std::io;

use mortal::{Style, Terminal};

fn main() -> io::Result<()> {
    let term = Terminal::new()?;

    term.write_styled(None, None, None,
        "normal text\n")?;

    term.write_styled(None, None, Style::BOLD,
        "bold text\n")?;

    term.write_styled(None, None, Style::ITALIC,
        "italic text\n")?;

    term.write_styled(None, None, Style::REVERSE,
        "reverse text\n")?;

    term.write_styled(None, None, Style::UNDERLINE,
        "underlined text\n")?;

    Ok(())
}
