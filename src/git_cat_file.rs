use std::process;

pub fn git_cat_file(args: &[String]) {
    let nb_args = args.len();
    if nb_args != 4 {
        eprintln!("Wrong number of args");
        process::exit(1);
    }
    match args[2].as_str() {
        "-p" => println!("IMPLEMENTED"),
        _ => {
            eprintln!("unknown command: {}", args[2]);
            process::exit(1);
        }
    }

    // let object_location = get_object_location(&args[3]).unwrap();
}
