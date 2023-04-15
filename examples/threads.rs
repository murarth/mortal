extern crate mortal;
extern crate rand;

use std::io;
use std::sync::Arc;
use std::thread::{spawn, sleep};
use std::time::Duration;

use mortal::{Color, Terminal};

use rand::{Rng, seq::SliceRandom, thread_rng};

// A unique color for each thread
const COLORS: &[Color] = &[
    Color::Blue,
    Color::Red,
    Color::Green,
    Color::Cyan,
    Color::Magenta,
];

fn main() -> io::Result<()> {
    // Wrapping the Terminal in an Arc allows us
    // to share ownership with multiple threads.
    let term = Arc::new(Terminal::new()?);

    // Join handles for spawned threads
    let mut handles = Vec::new();

    // Give a random color to each thread
    let mut colors = COLORS.to_vec();
    colors.shuffle(&mut thread_rng());

    for i in 0..5 {
        let name = format!("child{}", i);
        let term = term.clone();
        let color = colors[i];

        let handle = spawn(move || {
            run_task(&name, color, &term).unwrap()
        });

        handles.push(handle);
    }

    for handle in handles {
        handle.join().unwrap();
    }

    Ok(())
}

fn run_task(name: &str, color: Color, term: &Terminal)
        -> io::Result<()> {
    let mut rng = thread_rng();

    for _ in 0..5 {
        sleep(Duration::from_millis(rng.gen_range(100..300)));

        // Hold the lock while we perform a few different write operations.
        // This ensures that no other thread's output will interrupt ours.
        let mut lock = term.lock_write().unwrap();

        lock.write_str("[")?;
        lock.bold()?;
        lock.set_fg(color)?;
        lock.write_str(name)?;
        lock.clear_attributes()?;
        writeln!(lock, "]: random output: {}", rng.gen::<u8>())?;

        // The lock is dropped at the end of the loop,
        // giving other threads a chance to grab it.
    }

    Ok(())
}
