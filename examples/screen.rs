//! Example showing Screen usage

extern crate mortal;

use std::io;

use mortal::{Color, Event, Key, PrepareConfig, Screen, Style};

fn main() -> io::Result<()> {
    let screen = Screen::new(PrepareConfig{
        enable_keypad: true,
        enable_mouse: true,
        .. PrepareConfig::default()
    })?;

    draw_screen(&screen);
    screen.refresh()?;

    loop {
        if let Some(ev) = screen.read_event(None)? {
            if let Event::NoEvent = ev {
                continue;
            }

            if let Event::Key(Key::Char('q')) = ev {
                break;
            }

            draw_screen(&screen);
            screen.set_cursor((4, 0));
            write!(screen, "{:#?}", ev);

            screen.refresh()?;
        }
    }

    Ok(())
}

fn draw_screen(screen: &Screen) {
    screen.clear_screen();

    screen.write_at((0, 0), "Reading input. Press 'q' to stop.");

    screen.write_styled_at((2, 5),
        Color::Red, None, Style::BOLD, "Hello, world!");
}
