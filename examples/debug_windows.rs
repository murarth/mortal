extern crate mortal;

use std::io;
use mortal::Terminal;

fn main() -> io::Result<()> {
    Terminal::new()?.read_event(None)?;
    Ok(())
}
