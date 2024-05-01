use std::{
    io::{BufReader, Read},
    path::Path,
};

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

    // for more control, we pass the res in a bufreader

    // res starts with 0008NAK\nPACK
    let mut buf = vec![0; 12];
    res.read_exact(&mut buf)?;
    let val = String::from_utf8(buf).map_err(|e| Error::InvalidSmartHttpRes)?;
    if val != "0008NAK\nPACK" {
        return Err(Error::InvalidSmartHttpRes);
    }

    // then 4 bytes containing the version number
    // for ex [0,0,0,2]
    let mut buf = vec![0; 4];
    res.read_exact(&mut buf)?;

    // then 4 bytes containing the number of objects in the pack
    let mut buf = [0u8; 4];
    res.read_exact(&mut buf)?;
    // let nb_objects = buf[3] + buf[2] * 2 + buf[1] * 4 + buf[0] * 8;
    let nb_objects = u32::from_be_bytes(buf);

    dbg!(nb_objects);

    // create a bufreader for more control on the decoding
    let mut reader = BufReader::new(res);

    // then for the packfile itself, we iterate through all the objects
    for _ in 0..nb_objects {
        println!();

        // TODO: FInd why this works and the solution below does not
        // let mut size = 0usize;
        // let mut bitcount = 0usize;
        // loop {
        //     let mut v = [0u8; 1];
        //     reader.read_exact(&mut v)?;
        //     let tmp = (v[0] & 0b0111_1111) as usize;
        //     size += tmp << bitcount;
        //     bitcount += 7;
        //     if v[0] >> 7 == 0 {
        //         break;
        //     }
        // }
        //
        // let object_type = ((size >> 4) & 0b111) as u8;
        // let lower = size & 0b1111;
        // size >>= 7;
        // size <<= 4;
        // size += lower;
        // let cur_size = size;

        let mut buf = [0];
        reader.read_exact(&mut buf)?;
        let mut cur_byte = buf[0];
        // valid object types
        // - OBJ_COMMIT (1) - OBJ_TREE (2) - OBJ_BLOB (3) - OBJ_TAG (4) - OBJ_OFS_DELTA (6) - OBJ_REF_DELTA (7)
        let object_type = cur_byte >> 4 & 0b0111;
        dbg!(object_type);

        // then last 4 bits are part of size
        // Note that the size corresponds to the size of the uncompressed object
        // so we can not use it to read just the correct amount of bytes.
        // Fortunately, when decompressing with Zlib, it will stop automatically
        // at the EOF and we can then compare that the cur_size is equal to the
        // size of the buffer
        let mut cur_size = cur_byte & 0b1111;

        // while the MSB is 1,
        //  it means that the 7 lower bits of the next byte are part of the size
        let mut shift = 4;
        while cur_byte >= 128 {
            reader.read_exact(&mut buf)?;
            cur_byte = buf[0];
            let additional_size = (cur_byte & 0b01111111) << shift;
            shift += 7;
            cur_size += additional_size;
        }
        dbg!(cur_size);

        let mut buf = Vec::new();
        let mut z = flate2::bufread::ZlibDecoder::new(reader);
        // zlib will actually stop on EOF
        z.read_to_end(&mut buf)?;
        dbg!(buf.len());

        // TODO: There is either an error with the reasonning here
        // or the cur_size and the buf.len are not supposed to be always equal

        if buf.len() != cur_size as usize {
            // check that the uncompressed length corresponds to what was
            // mentionned in the packfile
            return Err(Error::IncorrectPackObjectSize {
                expected: cur_size as usize,
                got: buf.len(),
            });
        }

        if object_type == 1 || object_type == 3 {
            let s = String::from_utf8(buf).unwrap();
            dbg!(s);
        }

        //
        reader = z.into_inner();
    }
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
