//! Example of printing to the terminal via macros


#[macro_use] extern crate mortal;

use mortal::Terminal;
use mortal::Screen;

pub fn main() {
	let lock = Terminal::new().unwrap();
	let value = 42;
	let str_hello = "Hello";
	let x = mortal::Color::Blue;
	let str_world = "World";
	let a = "num";
	
	// Initial examples, locking, and 'lock' ambiguity tests
	term_writeln!(lock; "just term: " [red]("{} #{} {}!", str_hello, value, str_world)
		reset ("{}", 42) [bg=x]("XxX"));
	
	term_writeln!(lock lock; "lock term: ", red,("{} #{} {}!", str_hello, 2, 1)
		[reset] ("{}", 42) [bg=x]("XxX"));
	
	let term = lock;
	
	// Notice the 'lock' prefix creates a macro-local lock-guard named 'lock',
	// this examples shows that the macro-local guard does not interfere with
	// the surrounding scope.
	let lock = 420;
	term_writeln!(lock term; "lock again term: " bold red("#{}!", lock)
		reset (" {}", 42));
	
	{
		let mut guard = term.lock_write().unwrap();
		
		term_writeln!(guard; "just guard: " red ("{} #{} {}!", str_hello, value, lock)
			reset ("{}", 42) [bg=x]("XxX"));
		
		// Would deadlock at run time, because of it is already locked:
		//term_writeln!(lock term; "lock guard: " red("{} #{} {}!", str_hello, 2, 1)
		//	reset ("{}", 42) [bg=x] ("XxX"));
		
		// Would cause compile time error, because the guard has no lock function:
		//term_writeln!(lock guard; "lock guard: " red("{} #{} {}!", str_hello, 2, 1)
		//	reset ("{}", 42) [bg=x] ("XxX"));
	}
	
	// Stand alone versions
	term_write!(term ;);
	term_write!(  term  );
	term_writeln!(
		term
	);
	term_writeln!( term; );
	
	// Some primitive syntax without brackets
	term_write!(term; red "red");
	term_write!(term; blue "blue" green "green" reset "reset");
	term_write!(term; ,,;,;; " ");
	term_write!(term; blue "blue" #green "#gr" !fg "!fg" !bg "!bg" );
	term_write!(term; ;;;,;, " ");
	term_write!(term; bold "bold" underline "uline" red "red"
		!bold "!bold" !sty "!sty" );
	term_writeln!(term);
	
	// Some primitive syntax with brackets
	term_write!(term; [red] ("red"));
	term_write!(term; [blue] ("blue") [green] ("green") [reset] ("reset"));
	term_write!(term; ,,;,;; (" "));
	term_write!(term; [blue] ("blue") [#green] ("#gr") [!fg] ("!fg") [!bg] ("!bg") );
	term_write!(term; ;;;,;, (" "));
	term_write!(term; [bold] ("bold") [underline] ("uline") [red] ("red")
		[!bold] ("!bold") [!sty] ("!sty") );
	term_writeln!(term);
	
	// Printing
	term_writeln!(term; "St{i}rng" 42 true );
	term_writeln!(term; (:"St{i}rng") (:42) (:true) (: 40 + 2 == 42) );
	term_writeln!(term; (?"St{i}rng") (?42) (?true) (? 40 + 2 == 42) );
	term_writeln!(term; ("Hello Format") );
	term_writeln!(term; ("{}{}{}{}", "St{i}rng", 42, true, 40 + 2 == 42) );
	term_writeln!(term; ("{:?}{:?}{:?}{:?}", "St{i}rng", 42, true, 40 + 2 == 42) );
	term_writeln!(term);
	
	// All colors
	term_write!(term; black "black" blue "blue" cyan "cyan" green "green"
		magenta "magenta" red "red" white "white" yellow "yellow");
	term_write!(term; " - ");
	term_write!(term; #black "black" #blue "blue" #cyan "cyan" #green "green"
		#magenta "magenta" #red "red" #white "white" #yellow "yellow");
	term_writeln!(term);
	
	// All styles
	term_write!(term;
		bold "bold" underline "uline" reverse "rev" italic "italic" " - "
		!bold "!bold" !reverse "!rev" !italic "!italic" !underline "!uline");
	term_writeln!(term);
	
	// Resets
	term_write!(term;
		bold underline red #green "def" reset "reset");
	term_write!(term; " "
		bold underline red #green "def" !fg "!fg");
	term_write!(term; " "
		bold underline red #green "def" !bg "!bg");
	term_writeln!(term; " "
		bold underline red #green "def" !sty "!sty");
	
	// Stuff
	let theme = mortal::Theme{
		fg:Some(mortal::Color::Magenta), .. mortal::Theme::default()
	};
	term_write!(term; [=theme] "xae" bold ("s{}t", " Hi "));
	term_write!(term; [fg = mortal::Color::Red] "xae" bold ("s{}t", " Hi "));
	term_writeln!(term);
	for i in 0..=1 {
		term_write!(term; [fg = 
			if i == 0 {mortal::Color::Red} else {mortal::Color::Blue}]
		 "Colo" bold ("{}", i));
	}
	term_writeln!(term);
	let th = theme.clone();
	term_write!(term; [=th] ("xae") [bold] ("s{}t", " Hi "));
	term_writeln!(term);
	let r = mortal::Color::Red;
	let b = mortal::Color::Blue;
	term_write!(term; [fg=r] ("xae") [bg=b] ("s{}t", " Hi "));
	term_writeln!(term);
	let c1 = b;
	let c2 = r;
	term_write!(term; [fg=c1] ("xae") [bg=c2] ("s{}t", " Hi "));
	term_writeln!(term);
	term_write!(term; [fg=mortal::Color::Green] (:a));
	term_write!(term; [fg=c2] (?a));
	term_writeln!(term);
	term_write!(term; ,,;;;7,,;;;,;42,;;;1;,;,,;);
	term_writeln!(term);
	
	//term.refresh().unwrap();
	//term.wait_event(Some(std::time::Duration::from_millis(900))).unwrap();
	
}




