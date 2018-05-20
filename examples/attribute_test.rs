extern crate mortal;

use std::io;

use mortal::{Color, Style, Terminal};

fn main() -> io::Result<()> {
    let term = Terminal::new()?;

    write!(term, "plain")?;

    term.set_fg(Color::Red)?;
    write!(term, " add red fg")?;

    term.set_bg(Color::Blue)?;
    write!(term, " add blue bg")?;

    term.add_style(Style::BOLD)?;
    write!(term, " add bold")?;

    term.add_style(Style::REVERSE)?;
    write!(term, " add reverse")?;

    term.add_style(Style::UNDERLINE)?;
    write!(term, " add underline")?;

    term.add_style(Style::ITALIC)?;
    write!(term, " add italic")?;

    term.remove_style(Style::REVERSE)?;
    write!(term, " remove reverse")?;

    term.set_fg(Color::Magenta)?;
    write!(term, " add magenta fg")?;

    term.set_bg(Color::Green)?;
    write!(term, " add green bg")?;

    term.set_fg(None)?;
    write!(term, " reset fg")?;

    term.remove_style(Style::UNDERLINE)?;
    write!(term, " remove underline")?;

    term.remove_style(Style::BOLD)?;
    write!(term, " remove bold")?;

    term.set_bg(None)?;
    write!(term, " reset bg")?;

    term.remove_style(Style::ITALIC)?;
    write!(term, " remove italic")?;

    write!(term, "\n")
}
