extern crate mortal;

use std::io;

use mortal::{Color, Terminal};

fn main() -> io::Result<()> {
    let term = Terminal::new()?;

    if !term.supports_true_color() {
        drop(term);
        eprintln!("This terminal does not support true color");
        return Ok(());
    }

    for r in 0..8 {
        for g in 0..8 {
            for b in 0..8 {
                term.set_bg(Color::TrueColor{r, g, b})?;
                term.write_char('x')?;
            }
        }
    }

    term.write_char('\n')?;

    Ok(())
}
