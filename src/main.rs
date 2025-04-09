mod export;
mod session;
mod task;

use std::{env, path::Path, process};

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: {} <file_path>", args[0]);
        process::exit(1);
    }
    let path = Path::new(&args[1]);
    export::export_from(path).unwrap();
}
