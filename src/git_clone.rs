use std::{io::Read, path::Path};

use crate::{Error, Result};

/// see https://www.git-scm.com/docs/http-protocol
pub fn git_clone<P: AsRef<Path>>(repository_url: &str, directory: P) -> Result<()> {
    let client = reqwest::blocking::Client::new();

    let url = format!("{}/info/refs?service=git-upload-pack", repository_url);
    let mut res = client.get(url).send()?;

    // status code must be 200 or 304
    let status = res.status();
    if status != 200 && status != 304 {
        return Err(Error::InvalidSmartHttpRes);
    }

    // content type must be application/x-git-upload-pack-advertisement for smart http
    // we don't support dumb http protocol
    let content_type = res
        .headers()
        .get("content-type")
        .ok_or_else(|| Error::InvalidSmartHttpRes)?
        .to_str()
        .map_err(|_| Error::InvalidSmartHttpRes)?;

    if content_type != "application/x-git-upload-pack-advertisement" {
        return Err(Error::InvalidSmartHttpRes);
    };

    // 001e # service=git-upload-pack\n

    let initial_size = get_pkt_line_size(&mut res)?;
    let initial_content = get_pkt_line_content(&mut res, initial_size)?;

    if initial_content != "# service=git-upload-pack" {
        return Err(Error::InvalidSmartHttpRes);
    }

    // 0000
    let zero_size = get_pkt_line_size(&mut res)?;
    if zero_size != 0 {
        return Err(Error::InvalidSmartHttpRes);
    }

    let big_size = get_pkt_line_size(&mut res)?;
    let big_content = get_pkt_line_content(&mut res, big_size)?;

    // 004895dcfa3633004da0049d3d0fa03f80589cbcaf31 HEAD\0multi_ack thin-pack side-band\n
    let (head, tail) = big_content
        .split_once('\0')
        .ok_or_else(|| Error::InvalidSmartHttpRes)?;

    // not sure it ends with HEAD in all the cases
    // if !head.ends_with(" HEAD") {
    //     return Err(Error::InvalidSmartHttpRes);
    // }

    let head_hash = &head[..40];
    dbg!(head_hash);

    let attribs = tail.split(' ').collect::<Vec<_>>();
    dbg!(attribs);

    // Not sure I actually need them.
    // other rows only contain hash and ref
    let mut other_rows = Vec::new();
    loop {
        // 003fd049f6c27a2244e12041955e262a404c7faba355 refs/heads/master\n
        // 03c2cb58b79488a98d2721cea644875a8dd0026b115 refs/tags/v1.0\n
        // ...
        let size = get_pkt_line_size(&mut res)?;
        if size == 0 {
            break;
        }

        let content = get_pkt_line_content(&mut res, size)?;
        other_rows.push((size, content))
    }

    // no need to pass what we have, only what we want
    let url = format!("{}/git-upload-pack", repository_url);
    dbg!(&url);
    let pack_content = format!("0032want {}\n00000009done\n", head_hash);
    dbg!(&pack_content);
    let mut res = client
        .post(url)
        .header("Content-Type", "application/x-git-upload-pack-request")
        .body(pack_content)
        .send()?;

    println!("{}", res.text()?);

    Ok(())
}

pub fn get_pkt_line_size<R: Read>(reader: &mut R) -> Result<usize> {
    let size_encoding_len = 4;

    let mut size_buffer = vec![0; size_encoding_len];
    reader.read_exact(&mut size_buffer)?;
    let size = String::from_utf8(size_buffer).map_err(|_| Error::InvalidSmartHttpRes)?;
    let size = usize::from_str_radix(&size, 16).map_err(|_| Error::InvalidSmartHttpRes)?;
    Ok(size)
}

pub fn get_pkt_line_content<R: Read>(reader: &mut R, size: usize) -> Result<String> {
    let size_encoding_len = 4;

    let mut content_buffer = vec![0; size - size_encoding_len];
    reader.read_exact(&mut content_buffer)?;

    let mut content = String::from_utf8(content_buffer).map_err(|_| Error::InvalidSmartHttpRes)?;
    let last_char = content.pop().ok_or_else(|| Error::InvalidSmartHttpRes)?;
    if last_char != '\n' {
        return Err(Error::InvalidSmartHttpRes);
    }
    Ok(content)
}
