use std::error::Error;
use std::io::prelude::*;

use flate2::read::GzDecoder;

#[derive(PartialEq, Eq)]
pub enum FileType {
    KSM,
    KO,
    UNKNOWN,
}

/// Determines the type of a file using the raw bytes
pub fn determine_file_type(contents: &Vec<u8>) -> Result<FileType, Box<dyn Error>> {
    if is_gzip(contents) {
        let mut decoder = GzDecoder::new(&contents[..]);
        let mut decompressed = [0, 0, 0, 0];

        decoder.read_exact(&mut decompressed)?;

        if is_ksm(&decompressed) {
            return Ok(FileType::KSM);
        }
    } else if is_ko(contents) {
        return Ok(FileType::KO);
    }

    Ok(FileType::UNKNOWN)
}

/// Checks if the file is in proper GZIP format
fn is_gzip(contents: &[u8]) -> bool {
    contents[0] == 0x1f && contents[1] == 0x8b && contents[2] == 0x08 && contents[3] == 0x00
}

/// Checks the first 4 bytes of the file to tell if the contents are a KSM file or someone's compressed homework
fn is_ksm(contents: &[u8]) -> bool {
    contents[0] == 0x6b && contents[1] == 0x03 && contents[2] == 0x58 && contents[3] == 0x45
}

/// Checks the first 4 bytes of the file to tell if the contents are a KO file
fn is_ko(contents: &[u8]) -> bool {
    contents[0] == 0x6b && contents[1] == 0x01 && contents[2] == 0x6f && contents[3] == 0x66
}
