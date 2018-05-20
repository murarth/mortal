pub use self::screen::{
    Screen, ScreenReadGuard, ScreenWriteGuard,
};
pub use self::terminal::{
    PrepareState,
    Terminal, TerminalReadGuard, TerminalWriteGuard,
};

pub mod ext;
mod terminal;
mod screen;
