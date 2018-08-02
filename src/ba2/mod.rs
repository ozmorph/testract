use std::io::{Read, Seek, SeekFrom};
use std::path::PathBuf;

use byteorder::{LittleEndian, WriteBytesExt};

mod fallout4;
mod types;

use archive::{Archive, Extract};
use reader::{TESFile, TESReader};
use {Compression, Result};

// re-export only types that can be accessed from the main BSA structure
pub use self::types::{BA2File, BA2Header};

pub type BA2Archive = Archive<BA2Header, BA2File>;

/// Given a file path to a BSA file, opens and parses the archive into the generic BSA structure
pub fn from_file(path: PathBuf) -> Result<BA2Archive> {
    let mut reader = TESReader::from_file(&path)?;
    fallout4::parse_ba2(path, &mut reader)
}

impl Extract for BA2File {
    /// Given a file, extracts the file content from the BSA
    fn extract(&self, reader: &mut TESFile) -> Result<Vec<u8>> {
        match self.header {
            Some(_) => unimplemented!("Extraction is currently unimplemented for BA2 texture files"),
            None => {
                let general_file = &self.chunks[0];
                reader.seek(SeekFrom::Start(general_file.content_offset))?;
                let mut buffer_len = if general_file.compressed_size == 0 {
                    general_file.uncompressed_size
                } else {
                    general_file.compressed_size
                };
                let mut file_block = vec![0; buffer_len + 4];
                reader.read_exact(&mut file_block[4..])?;
                file_block.write_u64::<LittleEndian>(general_file.uncompressed_size as u64)?;
                if general_file.compressed_size != 0 {
                    Compression::Zlib.decompress_buffer(&file_block)
                } else {
                    Ok(file_block)
                }
            }
        }
    }
}
