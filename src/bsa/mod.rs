use std::io::{Read, Seek, SeekFrom};
use std::path::PathBuf;

use failure::ResultExt;

mod morrowind;
mod oblivion;
mod types;

use crate::archive::{Archive, Extract};
use crate::reader::{latin1_to_string, TESFile, TESReader};
use crate::{Compression, Result};

// reexports for documentation
pub use self::types::{BSAFile, BSAHeader};

pub type BSAArchive = Archive<BSAHeader, BSAFile>;

/// Given a file path to a BSA file, opens and parses the archive into the generic BSA structure
pub fn from_file(path: PathBuf) -> Result<BSAArchive> {
    let mut reader = TESReader::from_file(&path)?;

    let mut file_magic = [0; 4];
    reader
        .read_exact(&mut file_magic)
        .context("Unable to read BSA file identifier")?;
    let magic_str = latin1_to_string(&file_magic);
    match magic_str.as_ref() {
        "BSA\0" => oblivion::parse_bsa(path, &mut reader),
        "\x00\x01\x00\x00" => morrowind::parse_bsa(path, &mut reader),
        _ => unimplemented!("Unknown file id parsed"),
    }
}

impl Extract for BSAFile {
    /// Given a file, extracts the file content from the BSA
    fn extract(&self, reader: &mut TESFile) -> Result<Vec<u8>> {
        reader.seek(SeekFrom::Start(u64::from(self.offset)))?;
        let mut file_block = vec![0; self.size as usize];
        reader.read_exact(&mut file_block)?;

        if self.compression != Compression::None {
            if self.has_name {
                let compressed_slice_offset = (file_block[0] + 1) as usize;
                self.compression
                    .decompress_buffer(&file_block[compressed_slice_offset..])
            } else {
                self.compression.decompress_buffer(&file_block)
            }
        } else if self.has_name {
            let bstring_len: usize = (file_block[0] + 1) as usize;
            Ok(file_block[bstring_len..].to_vec())
        } else {
            Ok(file_block)
        }
    }
}
