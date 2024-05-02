use std::{
    collections::HashMap,
    io::{BufRead, BufReader, Read},
};

use crate::{
    git_object::{GitObject, GitObjectContent, TreeChild},
    Error, Result,
};

/// First thing to do before a git clone is to do a get request to
/// $GIT_URL/info/refs?service=git-upload-pack
/// A smart server response will look like this: see https://www.git-scm.com/docs/http-protocol
/// S: 200 OK
/// S: Content-Type: application/x-git-upload-pack-advertisement
/// S: Cache-Control: no-cache
/// S:
/// S: 001e# service=git-upload-pack\n
/// S: 0000
/// S: 004895dcfa3633004da0049d3d0fa03f80589cbcaf31 refs/heads/maint\0multi_ack\n
/// S: 003fd049f6c27a2244e12041955e262a404c7faba355 refs/heads/master\n
/// S: 003c2cb58b79488a98d2721cea644875a8dd0026b115 refs/tags/v1.0\n
/// S: 003fa3c2e2402b99163d1d59756e5f207ae21cccba4c refs/tags/v1.0^{}\n
/// S: 0000
///
///
#[derive(Debug)]
pub struct UploadPackDiscovery {
    pub repository_url: String,
    /// hash where HEAD points to, important for next steps
    pub head_hash: String,
    /// Extra parameters, found after \0 byte
    pub parameters: Vec<String>,
    /// contains other hash and ref name
    pub refs: Vec<(String, String)>,
}
impl UploadPackDiscovery {
    /// Size is encoded on 4 bytes, in hexadecimal
    pub fn get_line_size<R: Read>(reader: &mut R) -> Result<usize> {
        let size_encoding_len = 4;

        let mut size_buffer = vec![0; size_encoding_len];
        reader.read_exact(&mut size_buffer)?;
        let size = String::from_utf8(size_buffer).map_err(|_| Error::InvalidSmartHttpRes)?;
        let size = usize::from_str_radix(&size, 16).map_err(|_| Error::InvalidSmartHttpRes)?;
        Ok(size)
    }

    /// Content follows the size in the reader and ends with a \n
    pub fn get_line_content<R: Read>(reader: &mut R, size: usize) -> Result<String> {
        let size_encoding_len = 4;

        let mut content_buffer = vec![0; size - size_encoding_len];
        reader.read_exact(&mut content_buffer)?;

        let mut content =
            String::from_utf8(content_buffer).map_err(|_| Error::InvalidSmartHttpRes)?;
        let last_char = content.pop().ok_or_else(|| Error::InvalidSmartHttpRes)?;
        if last_char != '\n' {
            return Err(Error::InvalidSmartHttpRes);
        }
        Ok(content)
    }

    /// Builds the UploadPackDiscovery from a given repository url.
    /// Assumes smart http protocol is used
    pub fn from_repository_url(repository_url: &str) -> Result<Self> {
        let client = reqwest::blocking::Client::new();

        let url = format!("{}/info/refs?service=git-upload-pack", repository_url);
        let mut res = client.get(&url).send()?;

        // status code must be 200 or 304
        let status = res.status();
        if status != 200 && status != 304 {
            return Err(Error::InvalidDiscoveryUrl(url));
        }

        // content type must be application/x-git-upload-pack-advertisement for smart http
        // we don't support dumb http protocol
        let content_type = res
            .headers()
            .get("content-type")
            .ok_or_else(|| Error::ContentTypeNotFound)?
            .to_str()
            .map_err(|_| Error::ContentTypeInvalid)?;

        if content_type != "application/x-git-upload-pack-advertisement" {
            return Err(Error::WrongContentType {
                expected: "application/x-git-upload-pack-advertisement".to_string(),
                got: content_type.to_string(),
            });
        };

        // 001e # service=git-upload-pack\n

        let initial_size = Self::get_line_size(&mut res)?;
        let initial_content = Self::get_line_content(&mut res, initial_size)?;

        if initial_content != "# service=git-upload-pack" {
            return Err(Error::InvalidDiscoveryService {
                expected: "# service=git-upload-pack".to_string(),
                got: initial_content,
            });
        }

        // 0000
        let zero_size = Self::get_line_size(&mut res)?;
        if zero_size != 0 {
            return Err(Error::InvalidSmartHttpRes);
        }

        let big_size = Self::get_line_size(&mut res)?;
        let big_content = Self::get_line_content(&mut res, big_size)?;

        // 004895dcfa3633004da0049d3d0fa03f80589cbcaf31 HEAD\0multi_ack thin-pack side-band\n
        let (head, tail) = big_content
            .split_once('\0')
            .ok_or_else(|| Error::InvalidSmartHttpRes)?;

        // sometimes we have HEAD before \0, sometimes we have refs/heads/maint

        let head_hash = head[..40].to_owned();

        let parameters = tail.split(' ').map(|v| v.to_string()).collect::<Vec<_>>();

        // Not sure I actually need them.
        // other rows only contain hash and ref
        let mut refs = Vec::new();
        loop {
            // 003fd049f6c27a2244e12041955e262a404c7faba355 refs/heads/master\n
            // 03c2cb58b79488a98d2721cea644875a8dd0026b115 refs/tags/v1.0\n
            // ...
            let size = Self::get_line_size(&mut res)?;
            if size == 0 {
                break;
            }

            let content = Self::get_line_content(&mut res, size)?;
            let (hash, name) = content
                .split_once(' ')
                .ok_or_else(|| Error::InvalidSmartHttpRes)?;
            refs.push((hash.to_string(), name.to_string()));
        }
        Ok(Self {
            repository_url: repository_url.to_string(),
            parameters,
            head_hash,
            refs,
        })
    }
}

#[derive(Debug)]
pub enum GitPackObject {
    // - OBJ_COMMIT (1) - OBJ_TREE (2) - OBJ_BLOB (3) - OBJ_TAG (4) - OBJ_OFS_DELTA (6) - OBJ_REF_DELTA (7)
    Commit {
        content_bytes: Vec<u8>,
    },
    Tree {
        content_bytes: Vec<u8>,
    },
    Blob {
        content_bytes: Vec<u8>,
    },
    Tag {
        content_bytes: Vec<u8>,
    },
    RefDelta {
        base_object_hash: String,
        base_object_size: usize,
        reconstructed_object_size: usize,
        content_bytes: Vec<u8>,
    },
}

#[derive(Debug)]
pub struct GitPack {
    pack_objects: Vec<GitPackObject>,
}
impl GitPack {
    /// creates the most minimal pack content to send
    /// used in git clone
    pub fn create_minimal_pack_content_from_head_hash(head_hash: &str) -> String {
        format!("0032want {}\n00000009done\n", head_hash).to_string()
    }

    /// For most of the objects, in the first byte, we look at the MSB (leftmost bit).
    /// If it is 1 it means the next byte is also part of the size. If it is 0, it means
    /// we are on the last byte that is part of the size.
    /// For the first byte, the left bits 2,3,4 define the object so the size only starts
    /// on the last 4 bits.
    /// For ex 1001_1111 0010_1100 => type is 001, size is 0010_1100_1111
    /// valid object types are
    /// - OBJ_COMMIT (1) - OBJ_TREE (2) - OBJ_BLOB (3) - OBJ_TAG (4) - OBJ_OFS_DELTA (6) - OBJ_REF_DELTA (7)
    pub fn get_next_object_type_and_size<R: Read>(reader: &mut R) -> Result<(usize, usize)> {
        let mut buf = [0];
        reader.read_exact(&mut buf)?;
        let mut cur_byte = buf[0] as usize; // usize to avoid overflow when shifting

        let object_type = cur_byte >> 4 & 0b0111;
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
            cur_byte = buf[0] as usize;
            // the reason we need to cast as usize is to avoid overflow
            let additional_size = (cur_byte & 0b01111111) << shift;
            shift += 7;
            cur_size += additional_size;
        }
        Ok((object_type, cur_size))
    }

    /// same as get_next_object_type_and_size but adapted for cases where the
    /// type is not encoded in the first byte
    /// After OBS_REF_DELTA (7), we have the base object (20 bits)
    /// then the size of the base object and the size of the next object
    /// Here for the size encoding, we don't need to reserve bits for the type
    /// and so we can use 7 bits starting from the first byte
    pub fn get_next_size_without_type<R: Read>(reader: &mut R) -> Result<usize> {
        let mut buf = [0];
        let mut shift = 0;
        let mut cur_size = 0;

        loop {
            reader.read_exact(&mut buf)?;
            let cur_byte = buf[0] as usize;
            let additional_size = (cur_byte & 0b01111111) << shift;
            shift += 7;
            cur_size += additional_size;
            if cur_byte < 128 {
                break;
            }
        }
        Ok(cur_size)
    }

    pub fn find_highest_multiple_bit_pos(num: usize, multiple_bit: usize) -> usize {
        let mut pos = 0;
        let mut num = num;
        while num > 0 {
            pos += 1;
            num >>= multiple_bit;
        }
        pos
    }

    pub fn from_repository_url_and_pack_content(
        repository_url: &str,
        pack_content: &str,
    ) -> Result<Self> {
        let url = format!("{}/git-upload-pack", repository_url);
        let client = reqwest::blocking::Client::new();
        let mut res = client
            .post(url)
            .header("Content-Type", "application/x-git-upload-pack-request")
            .body(pack_content.to_string())
            .send()?;

        // res starts with 0008NAK\nPACK
        let mut buf = vec![0; 12];
        res.read_exact(&mut buf)?;
        let val = String::from_utf8(buf).map_err(|_| Error::InvalidSmartHttpRes)?;
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

        // create a bufreader for more control on the decoding
        let mut reader = BufReader::new(res);

        // then for the packfile itself, we iterate through all the objects
        let mut pack_objects = Vec::new();
        for _ in 0..nb_objects {
            println!();

            let (object_type, cur_size) = Self::get_next_object_type_and_size(&mut reader)?;

            match object_type {
                1..=4 => {
                    let mut buf = Vec::new();
                    let mut z = flate2::bufread::ZlibDecoder::new(reader);
                    // zlib will actually stop on EOF
                    z.read_to_end(&mut buf)?;

                    if buf.len() != cur_size {
                        // check that the uncompressed length corresponds to what was
                        // mentionned in the packfile
                        return Err(Error::IncorrectPackObjectSize {
                            expected: cur_size,
                            got: buf.len(),
                        });
                    }

                    let git_pack_object = match object_type {
                        1 => GitPackObject::Commit { content_bytes: buf },
                        2 => GitPackObject::Tree { content_bytes: buf },
                        3 => GitPackObject::Blob { content_bytes: buf },
                        4 => GitPackObject::Tag { content_bytes: buf },
                        _ => panic!("Pattern is unreachable"), // unreachable
                    };
                    pack_objects.push(git_pack_object);

                    // important to release the inner reader because it is moved in the
                    // zlib decoder.
                    reader = z.into_inner();
                }
                6 => todo!(),
                7 => {
                    // after the size, we get the base object name
                    let mut base_object = vec![0; 20];
                    reader.read_exact(&mut base_object)?;
                    dbg!(hex::encode(&base_object));

                    // then the diff as zlib compressed data
                    let mut buf = Vec::new();
                    let mut z = flate2::bufread::ZlibDecoder::new(reader);

                    let base_object_size = Self::get_next_size_without_type(&mut z)?;
                    let reconstructed_object_size = Self::get_next_size_without_type(&mut z)?;

                    // TODO: probably a better way to do it
                    // but we can easily retrieve the nb of bytes needed for the size as we use
                    // 7 bits per byte
                    let nb_bytes_base_object_size =
                        Self::find_highest_multiple_bit_pos(base_object_size, 7);
                    let nb_bytes_reconstructed_object_size =
                        Self::find_highest_multiple_bit_pos(reconstructed_object_size, 7);

                    z.read_to_end(&mut buf)?;

                    if buf.len() + nb_bytes_base_object_size + nb_bytes_reconstructed_object_size
                        != cur_size
                    {
                        return Err(Error::IncorrectPackObjectSize {
                            expected: cur_size,
                            got: buf.len(),
                        });
                    }

                    dbg!(
                        base_object_size,
                        reconstructed_object_size,
                        cur_size,
                        buf.len()
                    );
                    let git_pack_object = GitPackObject::RefDelta {
                        base_object_hash: hex::encode(base_object),
                        content_bytes: buf,
                        base_object_size,
                        reconstructed_object_size,
                    };
                    pack_objects.push(git_pack_object);
                    reader = z.into_inner();
                }
                x => Err(Error::InvalidPackObjectType(x))?,
            }
        }
        // TODO:Add checksum validation
        Ok(GitPack { pack_objects })
    }

    pub fn into_git_objects(self) -> Result<Vec<GitObject>> {
        let mut obj_map = HashMap::<String, usize>::new();

        let mut git_objects = Vec::new();

        for git_pack_object in self.pack_objects {
            match git_pack_object {
                GitPackObject::Blob { content_bytes } => {
                    let git_object = GitObject::from_blob_content_bytes(content_bytes)?;
                    obj_map.insert(git_object.hash.clone(), git_objects.len());
                    git_objects.push(git_object);
                }
                GitPackObject::Tree { content_bytes } => {
                    let git_object = GitObject::from_tree_content_bytes(content_bytes)?;
                    obj_map.insert(git_object.hash.clone(), git_objects.len());
                    git_objects.push(git_object);
                }
                GitPackObject::Commit { content_bytes } => {
                    let git_object = GitObject::from_commit_content_bytes(content_bytes)?;
                    obj_map.insert(git_object.hash.clone(), git_objects.len());
                    git_objects.push(git_object);
                }
                GitPackObject::Tag { .. } => {
                    println!("tag not supported");
                }
                GitPackObject::RefDelta {
                    base_object_hash,
                    content_bytes,
                    base_object_size,
                    reconstructed_object_size,
                } => {
                    if obj_map.contains_key(&base_object_hash) {
                        let real_base_object_size =
                            git_objects[*obj_map.get(&base_object_hash).unwrap()].size;
                        if real_base_object_size != base_object_size {
                            Err(Error::WrongObjectSize {
                                expected: real_base_object_size,
                                got: base_object_size,
                            })?;
                        }

                        // TODO: actual construction of the object

                        println!("{:08b}", content_bytes[0])
                    } else {
                        println!("STRANGE");
                    }
                }
                _ => {}
            }
        }
        let a = dbg!(obj_map
            .get("718a9b3efbd49b0c896cf632d81a6cdc6e673806")
            .unwrap());
        dbg!(&git_objects[*a]);
        Ok(git_objects)
    }
}
