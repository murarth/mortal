extern crate mortal;
extern crate rand;

use std::io;
use std::sync::Arc;
use std::sync::mpsc::{sync_channel, SyncSender, TrySendError};
use std::thread::{spawn, sleep};
use std::time::Duration;

use mortal::{Color, Cursor, Event, Key, PrepareConfig, Screen};

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
    // Wrapping the Screen in an Arc allows us
    // to share ownership with multiple threads.
    let screen = Arc::new(Screen::new(PrepareConfig{
        block_signals: true,
        .. PrepareConfig::default()
    })?);

    // Join handles for spawned threads
    let mut handles = Vec::new();
    // We use a "rendezvous" SyncChannel to signal termination to threads.
    // The main thread holds the reader and, when it is dropped, all threads
    // will exit.
    let (shutdown_tx, shutdown_rx) = sync_channel(0);

    // Give a random color to each thread
    let mut colors = COLORS.to_vec();
    colors.shuffle(&mut thread_rng());

    screen.write_at((0, 0), "Running threads. Press 'q' to stop.");
    screen.set_cursor((0, 0));
    screen.refresh()?;

    for i in 0..5 {
        let name = format!("child{}", i);
        let screen = screen.clone();
        let color = colors[i];
        let line = 2 + i * 2;
        let chan = shutdown_tx.clone();

        let handle = spawn(move || {
            run_task(&name, line, color, &screen, &chan).unwrap()
        });

        handles.push(handle);
    }

    // Only one thread will be reading events,
    // so we can hold the read lock for the duration of the program.
    let mut read = screen.lock_read().unwrap();

    loop {
        let ev = read.read_event(None)?;

        if let Some(Event::Key(Key::Char('q'))) = ev {
            break;
        }
    }

    // Signal threads to shutdown
    drop(shutdown_rx);

    for handle in handles {
        handle.join().unwrap();
    }

    Ok(())
}

fn run_task(name: &str, line: usize, color: Color, screen: &Screen,
        sender: &SyncSender<()>) -> io::Result<()> {
    let mut rng = thread_rng();

    loop {
        // Check whether the main thread is signalling an exit
        match sender.try_send(()) {
            Err(TrySendError::Disconnected(_)) => break,
            _ => ()
        }

        sleep(Duration::from_millis(rng.gen_range(300..500)));

        // Hold the lock while we perform a few different write operations.
        // This ensures that no other thread's output will interrupt ours.
        let mut lock = screen.lock_write().unwrap();

        lock.set_cursor(Cursor{line, column: 0});

        lock.write_str("[");
        lock.bold();
        lock.set_fg(color);
        lock.write_str(name);
        lock.clear_attributes();
        writeln!(lock, "]: random output: {:>3}", rng.gen::<u8>());

        // Set the cursor back to the starting point.
        // Without this, the cursor would jump around the screen after each write.
        lock.set_cursor((0, 0));
        lock.refresh()?;

        // The lock is dropped at the end of the loop,
        // giving other threads a chance to grab it.
    }

    Ok(())
}
