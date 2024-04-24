#[allow(unused_imports)]
use std::env;
#[allow(unused_imports)]
mod git_init;
use git_init::git_init;

fn main() {
    // Uncomment this block to pass the first stage
    let args: Vec<String> = env::args().collect();

    match args[1].as_str() {
        "init" => git_init(&args),
        "cat-file" => match args[2].as_str() {
            "-p" => println!("IMPLEMENTED"),
            _ => eprintln!("unknown command: {}", args[2]),
        },
        _ => eprintln!("unknown command: {}", args[1]),
    }
}
