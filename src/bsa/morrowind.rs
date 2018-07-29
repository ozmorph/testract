// Copyright Notice:   The Elder Scrolls, Morrowind, Oblivion, Skyrim, and Fallout are registered trademarks or
// trademarks of ZeniMax Media Inc.
//
//! Morrowind-style BSA archive bitflag, enum, and struct definitions and parsing functions
//!
//! BSA file structure documentation credit:    <http://en.uesp.net/wiki/Tes3Mod:BSA_File_Format#The_format>
//!
//! BSA file structure documentation license:   <https://creativecommons.org/licenses/by-sa/2.5/>
//!
//! ```
//! Morrowind-style BSA file structure
//! --------------------------------------------------------------------------------------------------------------
//! | header             | Header                           | Metadata for whole archive
//! | file_sizes_offsets | 8 * file_count                   | Size of file and offset in raw_data section
//! | name_offsets       | 4 * file_count                   | Offset of the filename in the name_block
//! | name_block         | hash_block - (12 * file_count)   | Lowercase ASCII, null-terminate file names
//! | hash_block         | 8 * file_Count                   | Hashes of the file names above
//! | raw_data           | (raw data)                       | Raw file data. Uncompressed and unseparated.
//! --------------------------------------------------------------------------------------------------------------
//! ```
use std::io::{Seek, SeekFrom};
use std::path::PathBuf;

use failure::ResultExt;
use nom::le_u32;

// top-level imports
use archive::FileMap;
use bsa::BSAArchive;
use reader::TESFile;
use {Compression, Result};

// bsa imports
use bsa::types::{ArchiveFlags, BSAFile, BSAHeader, FileFlags, Version};

/// All Morrowind-style BSA headers are 8 (0x8) bytes after parsing the file magic
const SERIALIZED_HEADER_LEN: usize = 0x8;
/// All Morrowind-style file records are 8 (0x8) bytes
const SERIALIZED_FILE_RECORD_LEN: usize = 0x8;

/// Creates a BSA object
pub fn parse_bsa(path: PathBuf, reader: &mut TESFile) -> Result<BSAArchive> {
    // Follows the Morrowind BSA file structure (described at the top of the file)

    // Read in the header
    let header = reader
        .parse_exact(SERIALIZED_HEADER_LEN, mw_bsa_header_parser)
        .context("Can't parse Morrowind style BSA header")?;

    // Read in the file records which contain the file size and file offset
    let file_records = reader
        .parse_exact(SERIALIZED_FILE_RECORD_LEN * header.file_count, parse_file_records)
        .context("Failed to parse Morrowind file records")?;

    // skip over the file name offset block as we don't need it
    reader.seek(SeekFrom::Current((4 * header.file_count) as i64))?;

    // get all of the file names by reading and parsing the bstring block
    let name_block_size = header.hash_offset - (12 * header.file_count); // calculation taken from BSA documentation
    let file_names = reader
        .parse_bstring_block(name_block_size)
        .context("Failed to read file name block")?;

    // Create a hashmap mapping file names => file metadata records to quickly grab file data from the BSA
    let file_hashmap = create_file_hashmap(&header, file_records, file_names);

    // Convert the header to a BSA header
    let bsa_header = BSAHeader {
        version:       Version::MORROWIND,
        archive_flags: ArchiveFlags::empty(),
        file_flags:    FileFlags::empty(),
        file_count:    header.file_count,
    };

    Ok(BSAArchive {
        path,
        header: bsa_header,
        file_hashmap,
    })
}

fn create_file_hashmap(
    header: &MWBSAHeader,
    file_records: Vec<MWFileRecord>,
    file_names: Vec<String>,
) -> FileMap<BSAFile> {
    // Zips the vector of file names up with the previous iterator
    let file_record_iter = file_records.into_iter().zip(file_names);

    // calculate the file data offset
    let file_data_offset: u32 = (header.hash_offset + (8 * header.file_count) + SERIALIZED_HEADER_LEN) as u32;

    // Iterates over each file and inserts it into a new hashmap
    let mut file_hashmap: FileMap<BSAFile> = Default::default();
    for (file_record, file_name) in file_record_iter {
        let bsa_file = BSAFile {
            has_name:    false,
            compression: Compression::None,
            size:        file_record.size,
            offset:      file_data_offset + file_record.offset,
        };
        file_hashmap.insert(PathBuf::from(file_name), bsa_file);
    }

    file_hashmap
}

/// Metadata for the whole archive.
///
/// Encoded format
/// ```
/// ------------------------
/// file_id         char[4]
/// hash_offset     u32
/// file_count      u32
/// ------------------------
/// ```
struct MWBSAHeader {
    /// Count of all files in the archive
    file_count: usize,
    /// Offset to the start of the hash block
    hash_offset: usize,
}

named!(mw_bsa_header_parser<&[u8], MWBSAHeader>,
    add_return_error!(ErrorKind::Custom(200),
        do_parse!(
            hash_offset:                le_u32 >>
            file_count:                 le_u32 >>
            (
                MWBSAHeader {
                    file_count: file_count as usize,
                    hash_offset: hash_offset as usize,
                }
            )
        )
    )
);

/// Metadata for a single file
///
/// Encoded format
/// ```
/// -----------
/// size    u32
/// offset  u32
/// -----------
/// ```
struct MWFileRecord {
    /// Size of the file data
    size: u32,
    /// Offset from file byte zero to the raw file data
    offset: u32,
}

named!(parse_file_records<&[u8], Vec<MWFileRecord>>,
    many0!(complete!(
        add_return_error!(ErrorKind::Custom(201),
            do_parse!(
                size:    le_u32 >>
                offset:  le_u32 >>
                (
                    MWFileRecord {
                        size,
                        offset,
                    }
                )
            )
        )
    ))
);
