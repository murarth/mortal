extern crate mortal;

use std::io;

use mortal::{Event, Key, Screen};

fn main() -> io::Result<()> {
    let screen = Screen::new(Default::default())?;

    let size = screen.size();

    if size.lines < 10 || size.columns < 20 {
        drop(screen);
        eprintln!("screen is too small");
        return Ok(());
    }

    writeln!(screen, "Testing Screen drawing");
    writeln!(screen, "Press 'q' to quit");

    screen.set_cursor((3, 0));
    writeln!(screen, "Ｆｕｌｌ Ｗｉｄｔｈ");

    screen.set_cursor((5, size.columns - 4));
    writeln!(screen, "wrapping text");

    screen.set_cursor((7, size.columns - 15));
    writeln!(screen, "Ｗｒａｐｐｉｎｇ ｆｕｌｌ ｗｉｄｔｈ");

    screen.set_cursor((9, 0));
    writeln!(screen, "Ｉｎｔｅｒｒｕｐｔｅｄ ｆｕｌｌ ｗｉｄｔｈ");

    screen.set_cursor((9, 15));
    write!(screen, "xxxx");

    screen.set_cursor((0, 0));
    screen.refresh()?;

    loop {
        let ev = screen.read_event(None)?;

        if let Some(Event::Key(Key::Char('q'))) = ev {
            break;
        }
    }

    Ok(())
}
