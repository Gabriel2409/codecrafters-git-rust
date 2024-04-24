use std::fs;
use std::process;
pub fn git_init(args: &[String]) -> () {
    let nb_args = args.len();
    if nb_args != 2 {
        eprintln!("Wrong number of args");
        process::exit(1);
    }
    fs::create_dir(".git").unwrap();
    fs::create_dir(".git/objects").unwrap();
    fs::create_dir(".git/refs").unwrap();
    fs::write(".git/HEAD", "ref: refs/heads/main\n").unwrap();
    println!("Initialized git directory");
}
