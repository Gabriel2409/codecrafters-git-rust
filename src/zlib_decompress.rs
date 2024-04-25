use flate2::bufread;
use std::fs::File;
use std::io::BufReader;
use std::io::Read;
use std::path::Path;

use crate::{Error, Result};

pub fn decompress<P: AsRef<Path>>(file_path: P) -> Result<String> {
    let input = BufReader::new(File::open(file_path)?);

    let mut decompressed_content = Vec::new();

    let mut decoder = bufread::ZlibDecoder::new(input);
    decoder.read_to_end(&mut decompressed_content)?;
    Ok(String::from_utf8_lossy(&decompressed_content).to_string())
}
