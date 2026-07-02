use rdc::{Memory, eval};
use rustyline::{Config, DefaultEditor, error::ReadlineError};
use std::io::Write;

fn main() {
    let mut memory = Memory::default();
    let out = &mut std::io::stdout().lock();
    let args: Vec<String> = std::env::args().collect();

    if args.len() == 1 {
        let config = Config::builder().auto_add_history(true).build();
        let Ok(mut reader) = DefaultEditor::with_config(config) else {
            eprintln!("Error: failed to initialize repl");
            std::process::exit(74);
        };
        loop {
            let line = match reader.readline("") {
                Ok(line) => line,
                Err(ReadlineError::Eof | ReadlineError::Interrupted) => return,
                Err(err) => {
                    eprintln!("Error: {}", err);
                    std::process::exit(74);
                }
            };
            eval(line.as_bytes(), &mut memory, out, false);
        }
    } else {
        if args[1] == "-h" || args[1] == "--help" {
            let help = "is a reverse-polish notation command-line calculator which supports unlimited precision arithmetic. It uses the same syntax as dc.";
            if writeln!(out, "Usage: {} [SCRIPT]..\n\n{} {}", args[0], args[0], help).is_err() {
                std::process::exit(74);
            }
            return;
        }
        for s in &args[1..] {
            if !eval(s.as_bytes(), &mut memory, out, true) {
                std::process::exit(1)
            }
        }
    }
}
