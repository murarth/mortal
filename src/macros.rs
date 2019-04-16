//! Provides macros easier printing with colors and styles.
//! See: https://github.com/murarth/mortal/issues/7




/// Writes on term
#[macro_export]
macro_rules! term_write {
	// Lock prefix
	( lock $term:ident; $($rest:tt)* ) => {
		// Scoped in order to force unlock at the end.
		// Otherwise, calling this method twice in row could deadlock.
		{
			let mut lock = $term.lock_write().unwrap();
			term_write!(lock; $( $rest )* );
		}
	};
	
	// Final rule
	( $term:ident $( ; )* ) => {
		$term.clear_attributes();
	};
	
	// Optional , and ; as separators
	( $term:ident; , $($rest:tt)* ) => {
		term_write!($term; $($rest)*);
	};
	( $term:ident; ; $($rest:tt)* ) => {
		term_write!($term; $($rest)*);
	};
	
	// Foreground Colors
	( $term:ident; black $($rest:tt)*) => {
		$term.set_fg($crate::Color::Black);
		term_write!($term; $($rest)*);
	};
	( $term:ident; blue $($rest:tt)*) => {
		$term.set_fg($crate::Color::Blue);
		term_write!($term; $($rest)*);
	};
	( $term:ident; cyan $($rest:tt)*) => {
		$term.set_fg($crate::Color::Cyan);
		term_write!($term; $($rest)*);
	};
	( $term:ident; green $($rest:tt)*) => {
		$term.set_fg($crate::Color::Green);
		term_write!($term; $($rest)*);
	};
	( $term:ident; magenta $($rest:tt)*) => {
		$term.set_fg($crate::Color::Magenta);
		term_write!($term; $($rest)*);
	};
	( $term:ident; red $($rest:tt)*) => {
		$term.set_fg($crate::Color::Red);
		term_write!($term; $($rest)*);
	};
	( $term:ident; white $($rest:tt)*) => {
		$term.set_fg($crate::Color::White);
		term_write!($term; $($rest)*);
	};
	( $term:ident; yellow $($rest:tt)*) => {
		$term.set_fg($crate::Color::Yellow);
		term_write!($term; $($rest)*);
	};
	
	// Background Colors
	( $term:ident; #black $($rest:tt)*) => {
		$term.set_bg($crate::Color::Black);
		term_write!($term; $($rest)*);
	};
	( $term:ident; #blue $($rest:tt)*) => {
		$term.set_bg($crate::Color::Blue);
		term_write!($term; $($rest)*);
	};
	( $term:ident; #cyan $($rest:tt)*) => {
		$term.set_bg($crate::Color::Cyan);
		term_write!($term; $($rest)*);
	};
	( $term:ident; #green $($rest:tt)*) => {
		$term.set_bg($crate::Color::Green);
		term_write!($term; $($rest)*);
	};
	( $term:ident; #magenta $($rest:tt)*) => {
		$term.set_bg($crate::Color::Magenta);
		term_write!($term; $($rest)*);
	};
	( $term:ident; #red $($rest:tt)*) => {
		$term.set_bg($crate::Color::Red);
		term_write!($term; $($rest)*);
	};
	( $term:ident; #white $($rest:tt)*) => {
		$term.set_bg($crate::Color::White);
		term_write!($term; $($rest)*);
	};
	( $term:ident; #yellow $($rest:tt)*) => {
		$term.set_bg($crate::Color::Yellow);
		term_write!($term; $($rest)*);
	};
	
	// Adding Style
	( $term:ident; bold $($rest:tt)*) => {
		$term.add_style($crate::Style::BOLD);
		term_write!($term; $($rest)*);
	};
	( $term:ident; italic $($rest:tt)*) => {
		$term.add_style($crate::Style::ITALIC);
		term_write!($term; $($rest)*);
	};
	( $term:ident; reverse $($rest:tt)*) => {
		$term.add_style($crate::Style::REVERSE);
		term_write!($term; $($rest)*);
	};
	( $term:ident; underline $($rest:tt)*) => {
		$term.add_style($crate::Style::UNDERLINE);
		term_write!($term; $($rest)*);
	};
	
	// Removing Style
	( $term:ident; !bold $($rest:tt)*) => {
		$term.remove_style($crate::Style::BOLD);
		term_write!($term; $($rest)*);
	};
	( $term:ident; !italic $($rest:tt)*) => {
		$term.remove_style($crate::Style::ITALIC);
		term_write!($term; $($rest)*);
	};
	( $term:ident; !reverse $($rest:tt)*) => {
		$term.remove_style($crate::Style::REVERSE);
		term_write!($term; $($rest)*);
	};
	( $term:ident; !underline $($rest:tt)*) => {
		$term.remove_style($crate::Style::UNDERLINE);
		term_write!($term; $($rest)*);
	};
	
	// Resets
	( $term:ident; reset $($rest:tt)*) => {
		$term.clear_attributes();
		term_write!($term; $($rest)*);
	};
	( $term:ident; !fg $($rest:tt)*) => {
		$term.set_fg(None);
		term_write!($term; $($rest)*);
	};
	( $term:ident; !bg $($rest:tt)*) => {
		$term.set_bg(None);
		term_write!($term; $($rest)*);
	};
	( $term:ident; !sty $($rest:tt)*) => {
		$term.set_style($crate::Style::default());
		term_write!($term; $($rest)*);
	};
	
	// Complex formats
	( $term:ident; [ fg = $e:expr ] $($rest:tt)*) => {
		$term.set_fg($e);
		term_write!($term; $($rest)*);
	};
	( $term:ident; [ bg = $e:expr ] $($rest:tt)*) => {
		$term.set_fg($e);
		term_write!($term; $($rest)*);
	};
	( $term:ident; [ sty = $e:expr ] $($rest:tt)*) => {
		$term.set_style($e);
		term_write!($term; $($rest)*);
	};
	( $term:ident; [ sty += $e:expr ] $($rest:tt)*) => {
		$term.add_style($e);
		term_write!($term; $($rest)*);
	};
	( $term:ident; [ sty -= $e:expr ] $($rest:tt)*) => {
		$term.remove_style($e);
		term_write!($term; $($rest)*);
	};
	( $term:ident; [ = $var:expr ] $($rest:tt)*) => {
		let x = &$var;
		let th = x as &$crate::Theme;
		$term.set_fg(th.fg).unwrap();
		$term.set_bg(th.bg).unwrap();
		$term.set_style(th.style).unwrap();
		term_write!($term; $($rest)*);
	};
	
	// Unwrap primitive formats from brackets []
	( $term:ident; [ ! $style:ident ] $($rest:tt)*) => {
		term_write!($term; ! $style $($rest)*);
	};
	( $term:ident; [ # $color:ident ] $($rest:tt)*) => {
		term_write!($term; # $color $($rest)*);
	};
	( $term:ident; [ $prim:ident ] $($rest:tt)*) => {
		term_write!($term; $prim $($rest)*);
	};
	
	// Single expressing printing
	( $term:ident; (: $e:expr ) $($rest:tt)*) => {
		write!($term, "{}", $e ).unwrap();
		term_write!($term; $($rest)*);
	};
	( $term:ident; (? $e:expr ) $($rest:tt)*) => {
		write!($term, "{:?}", $e ).unwrap();
		term_write!($term; $($rest)*);
	};
	
	// Format printing
	( $term:ident; ( $( $fmt:tt )+ ) $($rest:tt)*) => {
		write!($term, $( $fmt )+ ).unwrap();
		term_write!($term; $($rest)*);
	};
	
	// Literal printing
	( $term:ident; $s:tt $($rest:tt)*) => {
		write!($term, "{}", concat!($s)).unwrap();
		term_write!($term; $($rest)*);
	};
}


/// Same as term_write but adds a new line at the end.
#[macro_export]
macro_rules! term_writeln {
	( lock $term:ident; $( $rest:tt )* ) => {
		// Scoped in order to force unlock at the end.
		// Otherwise, calling this method twice in row could deadlock.
		{
			let mut lock = $term.lock_write().unwrap();
			term_write!(lock; $( $rest )* );
			writeln!(lock).unwrap();
		}
	};
	( $term:ident $( ; )* ) => {
		writeln!($term).unwrap();
	};
	( $term:ident; $( $rest:tt )* ) => {
		term_write!($term; $( $rest )* );
		writeln!($term).unwrap();
	};
}


