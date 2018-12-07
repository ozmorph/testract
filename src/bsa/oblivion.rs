// Copyright Notice:   The Elder Scrolls, Morrowind, Oblivion, Skyrim, and Fallout are registered trademarks or
// trademarks of ZeniMax Media Inc.
//
//! Oblivion-style BSA archive bitflag, enum, and struct definitions and parsing functions
//!
//! BSA file structure documentation credit:    <http://en.uesp.net/wiki/Tes5Mod:Archive_File_Format#File_Structure>
//!
//! BSA file structure documentation license:   <https://creativecommons.org/licenses/by-sa/2.5/>
//!
//! ```
//! Oblivion-style BSA file structure
//! --------------------------------------------------------------------------------------------------------------
//! | header            | Header                            | Metadata for whole archive
//! | folder_records    | FolderRecord[folder_count]        | Metadata for folders (count + offset)
//! | file_record_blocks| BSAFileRecordBlock[folder_count]  | Data for each folder (name + file records)
//! | file_name_block   | String                            | Optional. A block of lowercase file names
//! | files             | RawFileBlock[file_count]          | Raw file data that is optionally compressed
//! --------------------------------------------------------------------------------------------------------------
//! ```
use std::iter;
use std::path::{Path, PathBuf};

use failure::ResultExt;
use nom::{le_u32, le_u64};

// top-level imports
use crate::archive::FileMap;
use crate::reader::TESFile;
use crate::{Compression, Result};

// bsa imports
use crate::bsa::types::*;
use crate::bsa::BSAArchive;

/// All Oblivion-style BSA headers are the same size in serialized form, 32 (0x20), after parsing the file magic
const SERIALIZED_HEADER_LEN: usize = 0x20;
/// All Oblivion-style BSA file record blocks are the same size in serialized form, 16 (0x10)
const SERIALIZED_FILE_RECORD_LEN: usize = 0x10;
/// All Oblivion-style BSA folder records are the same size in serialized form, 16 (0x10)
const SERIALIZED_OB_FOLDER_RECORD_LEN: usize = 0x10;
/// The Skyrim Special Edition-style BSA folder records has a unique size in serialized form, 24 (0x18)
const SERIALIZED_SSE_FOLDER_RECORD_LEN: usize = 0x18;

/// Creates a BSA object
pub fn parse_bsa(path: PathBuf, mut reader: &mut TESFile) -> Result<BSAArchive> {
    // Follows the Oblivion BSA file structure (described at the top of the file)

    // Read in the header
    let header = reader
        .parse_exact(SERIALIZED_HEADER_LEN, ob_bsa_header_parser)
        .context("Can't parse Oblivion style BSA header")?;

    // Read in the file record blocks which each contain a variable number of file records
    let folders =
        read_file_record_blocks(&mut reader, header.folder_count, &header).context("Failed to read folder records")?;

    // If the archive flags indicated that the file name block exists, use it
    let file_names = if header.archive_flags.contains(ArchiveFlags::INCLUDE_FILE_NAMES) {
        reader
            .parse_bstring_block(header.total_file_name_length as usize)
            .context("Failed to read file name block")?
    } else {
        unimplemented!("Parsing BSA files without the INCLUDE_FILE_NAMES archive flag is currently unsupported");
    };

    // Create a hashmap mapping file names => file metadata records to quickly grab file data from the BSA
    let file_hashmap = create_file_hashmap(&header, folders, file_names);

    // Convert the header to a BSA header
    let bsa_header = BSAHeader {
        version:       header.version,
        archive_flags: header.archive_flags,
        file_flags:    header.file_flags,
        file_count:    header.file_count,
    };

    Ok(BSAArchive {
        path,
        header: bsa_header,
        file_hashmap,
    })
}

fn read_file_record_blocks(
    reader: &mut TESFile,
    num_folders: usize,
    header: &OBBSAHeader,
) -> Result<Vec<OBFolderRecord>> {
    // Read the folder metadata block which tells us how many files are in each folder
    // Skyrim Special Edition has a different header from the other formats
    let folder_metadata = if header.version == Version::SKYRIMSE {
        reader
            .parse_exact(
                SERIALIZED_SSE_FOLDER_RECORD_LEN * num_folders,
                sse_folder_metadata_parser,
            )
            .context("Failed parsing the SSE-style folder metadata block")?
    } else {
        reader
            .parse_exact(SERIALIZED_OB_FOLDER_RECORD_LEN * num_folders, ob_folder_metadata_parser)
            .context("Failed parsing the Oblivion-style folder metadata block")?
    };

    let mut file_record_blocks: Vec<OBFolderRecord> = Vec::with_capacity(num_folders);
    for metadata in folder_metadata {
        // The folder name is stored as a bzstring: byte-length prefixed and '\0' terminated
        let name = reader.parse_bzstring().context("Failed parsing a folder name")?;

        // Read out the file records
        let file_records = reader
            .parse_exact(SERIALIZED_FILE_RECORD_LEN * metadata.count, ob_file_records_parser)
            .context("Failed parsing file records")?;

        file_record_blocks.push(OBFolderRecord { name, file_records });
    }

    Ok(file_record_blocks)
}

fn create_file_hashmap(
    header: &OBBSAHeader,
    folders: Vec<OBFolderRecord>,
    file_names: Vec<String>,
) -> FileMap<BSAFile> {
    // Converts the vector of BSAFolderRecords into an iterator of (folder_name, file_record) to be more easily consumed
    let folder_file_iter = folders
        .into_iter()
        .flat_map(|folder| iter::repeat(folder.name).zip(folder.file_records.into_iter()));

    // Zips the vector of file names up with the previous iterator
    let folder_file_name_iter = file_names.into_iter().zip(folder_file_iter);

    // Iterates over each file and inserts it into a new hashmap
    let mut file_hashmap: FileMap<BSAFile> = Default::default();
    for (file_name, (folder_name, file_record)) in folder_file_name_iter {
        // Documentation on the Unofficial Elder Scrolls Pages (UESP) wiki seems to be wrong.
        // Even if the EMBED_FILE_NAMES flag is set on the archive, the file names are not found
        // in the individual file blocks. Therefore we always say false for Oblivion BSAs
        let has_name =
            header.archive_flags.contains(ArchiveFlags::EMBED_FILE_NAMES) && header.version != Version::OBLIVION;

        let mut is_compressed = header.archive_flags.contains(ArchiveFlags::COMPRESSED_ARCHIVE);
        is_compressed = if !file_record.uses_default_compression {
            !is_compressed
        } else {
            is_compressed
        };

        // For Skyrim Special Edition, Bethesda replaced Zlib compression with LZ4 compression.
        // Personal opinion: this is probably because LZ4 is multithread capable and thus lended
        // itself well to the newer console generation that has multiple cores to help load assets
        // and lower in-game load screens which have plagued console performance in the past
        let compression = if is_compressed {
            match header.version {
                Version::OBLIVION | Version::SKYRIM => Compression::Zlib,
                Version::SKYRIMSE => Compression::Lz4,
                _ => Compression::None,
            }
        } else {
            Compression::None
        };

        let bsa_file = BSAFile {
            has_name,
            compression,
            size: file_record.size,
            offset: file_record.offset,
        };
        file_hashmap.insert(Path::new(&folder_name).join(&file_name), bsa_file);
    }

    file_hashmap
}

/// Metadata for the whole archive.
///
/// Used by Oblivion, Fallout 3, Fallout New Vegas, Skyrim, and Skyrim Special Edition
///
/// ```
/// Encoded format
/// ----------------------------------
/// file_id                   char[4]
/// version                   u32
/// offset                    u32
/// archive_flags             u32
/// folder_count              u32
/// file_count                u32
/// total_folder_name_length  u32
/// total_file_name_length    u32
/// file_flags                u32
/// ----------------------------------
/// ```
struct OBBSAHeader {
    /// A single byte indicating the version of the file-format
    version: Version,
    /// A list of archive flags indicating how to interpret following records and data
    archive_flags: ArchiveFlags,
    /// Count of all folders in the archive
    folder_count: usize,
    /// Count of all files in the archive
    file_count: usize,
    /// Total length of all file names, including \0's.
    total_file_name_length: u32,
    /// List of flags specifying the type of files containing within the archive
    file_flags: FileFlags,
}

named!(ob_bsa_header_parser<&[u8], OBBSAHeader>,
    add_return_error!(ErrorKind::Custom(100),
        do_parse!(
            version:            version_parser >>
            _offset:                  take!(4) >>
            archive_flags: parse_archive_flags >>
            folder_count:               le_u32 >>
            file_count:                 le_u32 >>
            _total_folder_name_length:take!(4) >>
            total_file_name_length:     le_u32 >>
            file_flags:       parse_file_flags >>
            _unknown_bytes:           take!(2) >>
            (
                OBBSAHeader {
                    version,
                    archive_flags,
                    folder_count: folder_count as usize,
                    file_count: file_count as usize,
                    total_file_name_length,
                    file_flags,
                }
            )
        )
    )
);

/// Metadata for a single folder
///
/// ```
/// Encoded format (for Oblivion, Fallout 3, Fallout NV, Skyrim)
/// ------------------
/// name_hash   hash
/// count       u32
/// offset      u32
/// ------------------
/// ```
///
/// ```
/// Encoded format (for Skyrim Special Edition)
/// ------------------
/// name_hash   hash
/// count       u32
/// unknown     u32
/// offset      u32
/// unknown     u32
/// ------------------
/// ```
struct OBFolderMetadata {
    /// Number of files contained in this folder
    count: usize,
}

named!(ob_folder_metadata_parser<&[u8], Vec<OBFolderMetadata>>,
    many0!(complete!(
        add_return_error!(ErrorKind::Custom(101),
            do_parse!(
                _name_hash:     le_u64 >>
                file_count:     le_u32 >>
                _offset:        le_u32 >>
                (
                    OBFolderMetadata {
                        count: file_count as usize
                    }
                )
            )
        )
    ))
);

named!(sse_folder_metadata_parser<&[u8], Vec<OBFolderMetadata>>,
    many0!(complete!(
        add_return_error!(ErrorKind::Custom(102),
            do_parse!(
                _name_hash:     le_u64 >>
                file_count:     le_u32 >>
                _unknown:     take!(4) >>
                _offset:        le_u32 >>
                _unknown2:    take!(4) >>
                (
                    OBFolderMetadata {
                        count: file_count as usize
                    }
                )
            )
        )
    ))
);

/// Data for a single folder
///
/// Encoded format when [`ArchiveFlags`]::[`INCLUDE_DIR_NAMES`] is set to 1
/// ```
/// -------------------------------------------
/// | name            | bzstring              |
/// | file_records    | Vec<BSAFileRecord>    |
/// -------------------------------------------
/// ```
///
/// Encoded format when [`ArchiveFlags`]::[`INCLUDE_DIR_NAMES`] is set to 0
/// ```
/// -------------------------------------------
/// | file_records    | Vec<BSAFileRecord>    |
/// -------------------------------------------
/// ```
///
/// [`ArchiveFlags`]: struct.ArchiveFlags.html
/// [`INCLUDE_DIR_NAMES`]: struct.ArchiveFlags.html#associatedconstant.INCLUDE_DIR_NAMES
struct OBFolderRecord {
    /// Name of the folder
    name: String,
    /// A variable number of file records determined by the count field in [`BSAFileRecord`]
    ///
    /// [`BSAFileRecord`]: struct.BSAFileRecord.html
    file_records: Vec<OBFileRecord>,
}

/// Metadata for a single file
///
/// Encoded format
/// ```
/// -----------------------
/// | name_hash   | hash  |
/// | size        | u32   |
/// | offset      | u32   |
/// -----------------------
/// ```
struct OBFileRecord {
    /// Decides whether or not the file is compressed
    uses_default_compression: bool,
    /// Size of the file data
    size: u32,
    /// Offset from file byte zero to the raw file data
    offset: u32,
}

named!(ob_file_records_parser<&[u8], Vec<OBFileRecord>>,
    many1!(complete!(
        add_return_error!(ErrorKind::Custom(103),
            do_parse!(
                _name_hash:   take!(8) >>
                size:           le_u32 >>
                offset:         le_u32 >>
                (
                    OBFileRecord {
                        // If the (1<<30) bit of the size field is set to 1:
                        //   * and [`ArchiveFlags`]::[`COMPRESSED_ARCHIVE`] is set, this file is not compressed
                        //   * and [`ArchiveFlags`]::[`COMPRESSED_ARCHIVE`] is not set, this file is compressed
                        uses_default_compression: !(((size & 0x4000_0000) >> 30) == 1),
                        size:                         size & 0x3fff_ffff,
                        offset,
                    }
                )
            )
        )
    ))
);
