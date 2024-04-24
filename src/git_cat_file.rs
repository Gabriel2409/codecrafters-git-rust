use crate::{Error, Result};

pub fn git_cat_file(args: &[String]) -> Result<()> {
    let nb_args = args.len();
    if nb_args != 4 {
        return Err(Error::InvalidNbArgs {
            expected: 4,
            got: nb_args,
        });
    }
    match args[2].as_str() {
        "-p" => println!("IMPLEMENTED"),
        x => {
            return Err(Error::UnknownArgument(x.to_owned()));
        }
    }
    Ok(())

    // let object_location = get_object_location(&args[3]).unwrap();
}
