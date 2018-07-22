extern crate mortal;

use std::io;

fn main() -> io::Result<()> {
    run_example()
}

#[cfg(not(unix))]
fn run_example() -> io::Result<()> {
    eprintln!("This example demonstrates functionality specific to Unix platforms.");
    Ok(())
}

#[cfg(unix)]
fn run_example() -> io::Result<()> {
    use std::env::args;

    use mortal::Terminal;
    use mortal::unix::OpenTerminalExt;

    let path = match args().nth(1) {
        Some(path) => path,
        None => {
            eprintln!("No path specified. Try using /dev/tty");
            return Ok(());
        }
    };

    let term = Terminal::from_path(&path)?;

    writeln!(term, "Hello, terminal!")?;

    Ok(())
}
