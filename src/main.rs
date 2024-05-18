use std::{env, io, process};

fn main() {
    let config = tapeworm::Config::build(env::args()).unwrap_or_else(|e| {
        eprintln!("Problem parsing arguments: {}", e);
        process::exit(1);
    });

    if let Err(e) = tapeworm::run(config, io::stdin().lock()) {
        eprintln!("Application error: {}", e);
        process::exit(1);
    }
}
