use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};

use byteorder::{ByteOrder, LittleEndian};
use failure::ResultExt;
use flate2::read::ZlibDecoder;
use lz4;

mod morrowind;
mod oblivion;
mod types;

use reader::{latin1_to_string, TESFile, TESReader};
use Result;

// re-export only types that can be accessed from the main BSA structure
pub use self::types::{ArchiveFlags, BSAFile, BSAHeader, FileFlags, Version, BSA};

enum CompressionType {
    NONE,
    ZLIB,
    LZ4,
}

impl BSA {
    /// Given a file path to a BSA file, opens and parses the archive into the generic BSA structure
    pub fn from_file(path: PathBuf) -> Result<BSA> {
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

    /// Given a file path, extracts the file content from the BSA
    pub fn extract_via_name(&self, reader: &mut TESFile, file_path: &Path) -> Result<Vec<u8>> {
        let file_record = self.file_hashmap
            .get(file_path)
            .ok_or_else(|| format_err!("File {:#?} not found", file_path))?;
        self.extract_via_file(reader, file_record)
    }

    /// Given a file, extracts the file content from the BSA
    pub fn extract_via_file(&self, reader: &mut TESFile, file: &BSAFile) -> Result<Vec<u8>> {
        reader.seek(SeekFrom::Start(u64::from(file.offset)))?;
        let mut file_block = vec![0; file.size as usize];
        reader.read_exact(&mut file_block)?;

        // Documentation on the Unofficial Elder Scrolls Pages (UESP) wiki seems to be wrong.
        // Even if the EMBED_FILE_NAMES flag is set on the archive, the file names are not found
        // in the individual file blocks. Therefore we always say false for Oblivion BSAs
        let has_name = self.header.archive_flags.contains(ArchiveFlags::EMBED_FILE_NAMES)
            && self.header.version != Version::OBLIVION;

        // For Skyrim Special Edition, Bethesda replaced Zlib compression with LZ4 compression.
        // Personal opinion: this is probably because LZ4 is multithread capable and thus lended
        // itself well to the newer console generation that has multiple cores to help load assets
        // and lower in-game load screens which have plagued console performance in the past
        let compression_type = match self.header.version {
            Version::OBLIVION | Version::SKYRIM => CompressionType::ZLIB,
            Version::SKYRIMSE => CompressionType::LZ4,
            Version::MORROWIND => CompressionType::NONE,
            _ => unimplemented!("Decompression is currently not supported for this file type"),
        };

        if file.compressed {
            if has_name {
                let compressed_slice_offset = (file_block[0] + 1) as usize;
                decompress_buffer(&file_block[compressed_slice_offset..], &compression_type)
            } else {
                decompress_buffer(&file_block, &compression_type)
            }
        } else if has_name {
            let bstring_len: usize = (file_block[0] + 1) as usize;
            Ok(file_block[bstring_len..].to_vec())
        } else {
            Ok(file_block)
        }
    }
}

fn decompress_buffer(buffer: &[u8], compression_type: &CompressionType) -> Result<Vec<u8>> {
    let (length, data) = buffer.split_at(4);
    let uncompressed_length = LittleEndian::read_u32(length);
    let mut out_buffer = Vec::with_capacity(uncompressed_length as usize);
    match compression_type {
        CompressionType::ZLIB => {
            let mut decoder = ZlibDecoder::new(data);
            decoder
                .read_to_end(&mut out_buffer)
                .context("Unable to decompress ZLIB data")?;
        }
        CompressionType::LZ4 => {
            let mut decoder = lz4::Decoder::new(data)?;
            decoder
                .read_to_end(&mut out_buffer)
                .context("Unable to decompress LZ4 data")?;
        }
        CompressionType::NONE => out_buffer = data.to_vec(),
    };
    Ok(out_buffer)
}
