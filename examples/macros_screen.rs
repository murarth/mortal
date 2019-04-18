//! Example of printing to the screen via macros

#[macro_use] extern crate mortal;

use std::io;

use mortal::{Color, Style, Theme, Screen, Event, Key};

pub fn main() -> io::Result<()> {
	let screen = Screen::new(Default::default())?;

    let razzle_dazzle = Color::Red;
    let flooby_doo = Color::Blue;
    let zamabamafoo = Style::BOLD;
    let glitter_exploding = Theme::new(razzle_dazzle, flooby_doo, zamabamafoo);

    term_writeln!(screen, "Press 'q' to exit.");

    term_writeln!(screen);

    term_writeln!(screen,
        [black] "black "
        [blue] "blue "
        [cyan] "cyan "
        [green] "green "
        [magenta] "magenta "
        [red] "red "
        [white] "white "
        [yellow] "yellow"
        [reset]);

    term_writeln!(screen,
        [#black] "black "
        [#blue] "blue "
        [#cyan] "cyan "
        [#green] "green "
        [#magenta] "magenta "
        [#red] "red "
        [#white] "white "
        [#yellow] "yellow"
        [reset]);

    term_writeln!(screen,
        [bold] "bold " [!bold]
        [italic] "italic " [!italic]
        [reverse] "reverse " [!reverse]
        [underline] "underline" [!underline]);

    term_writeln!(screen,
        [fg=razzle_dazzle] "razzle dazzle " [!fg]
        [bg=flooby_doo] "flooby doo " [!bg]
        [style=zamabamafoo] "zamabamafoo!\n" [!style]
        [theme=glitter_exploding] "Like glitter is exploding inside me!" [reset]);

    term_writeln!(screen,
        ("foo {}", 42) " "
        (: "bar") " "
        (? "baz") " "
        "quux");

    screen.refresh()?;

    loop {
        if let Some(Event::Key(Key::Char('q'))) = screen.read_event(None)? {
            break;
        }
    }

    Ok(())
}
