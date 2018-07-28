use std::io::{Seek, SeekFrom};
use std::path::PathBuf;

use failure::ResultExt;
use nom::{le_u16, le_u32, le_u64, le_u8};

// top-level imports
use reader::TESFile;
use Result;

// BA2 imports
use ba2::types::*;

/// All BA2 headers are 24 (0x18) bytes
const HEADER_LEN: usize = 0x18;
/// All BA2 general file records are 36 (0x24) bytes
const GENERAL_FILE_LEN: usize = 0x24;
/// All BA2 texture header records are 24 (0x18) bytes
const TEXTURE_HEADER_LEN: usize = 0x18;
/// All BA2 texture chunk records are 24 (0x18) bytes
const TEXTURE_CHUNK_LEN: usize = 0x18;

/// Creates a BA2 object
pub fn parse_ba2(path: PathBuf, reader: &mut TESFile) -> Result<BA2> {
    // Read in the header
    let header = reader
        .parse_exact(HEADER_LEN, fo4_header_parser)
        .context("Can't parse a Fallout 4 .ba2 header")?;

    // Seek to the name table
    reader.seek(SeekFrom::Start(header.name_table_offset))?;
    let mut name_vec: Vec<PathBuf> = Vec::with_capacity(header.file_count);
    for _ in 0..header.file_count {
        let file_name = reader
            .parse_long_bstring()
            .context("Can't parse a Fallout 4 file path")?;
        name_vec.push(PathBuf::from(file_name));
    }

    // Seek to the beginning of the file info section
    reader.seek(SeekFrom::Start(u64::from((HEADER_LEN) as u32)))?;

    // Collect metadata about all of the files in the archive
    let files: Vec<BA2File> = match header.file_type {
        BA2Type::General => reader.parse_exact(GENERAL_FILE_LEN * header.file_count, fo4_general_files_parser)?,
        BA2Type::Textures => {
            let mut files: Vec<BA2File> = Vec::with_capacity(header.file_count);
            for _ in 0..header.file_count {
                let tex_header = reader.parse_exact(TEXTURE_HEADER_LEN, fo4_texture_header_parser)?;
                let tex_chunks =
                    reader.parse_exact(TEXTURE_CHUNK_LEN * tex_header.num_chunks, fo4_texture_chunks_parser)?;
                files.push(BA2File {
                    header: Some(tex_header),
                    chunks: tex_chunks,
                });
            }
            files
        }
    };

    // Create a hashmap mapping file names => file metadata records to quickly grab file data from the BA2
    let file_iter = name_vec.into_iter().zip(files.into_iter());
    let mut file_hashmap: FileMap = Default::default();
    for (file_name, file) in file_iter {
        file_hashmap.insert(file_name, file);
    }

    Ok(BA2 {
        path,
        header,
        file_hashmap,
    })
}

/// Parses metadata for the entire Fallout 4 .ba2 archive
///
/// Encoded format
/// ```
/// ------------------------
/// version             u32
/// file_type           char[4]
/// file_count          u32
/// name_table_offset   u32
/// ------------------------
/// ```
named!(fo4_header_parser<&[u8], BA2Header>,
    add_return_error!(ErrorKind::Custom(300),
        do_parse!(
            _file_magic:          tag!("BTDX") >>
            version:            version_parser >>
            file_type:             type_parser >>
            file_count:                 le_u32 >>
            name_table_offset:          le_u64 >>
            (
                BA2Header {
                    version,
                    file_type,
                    file_count: file_count as usize,
                    name_table_offset,
                }
            )
        )
    )
);

/// Parses all of the file metadata for a general .ba2 archive
///
/// Encoded format for non-textures
/// ```
/// ------------------------
/// name_hash           u32
/// extension           char[4]
/// dir_hash            u32
/// unknown_flags       u32
/// content_offset      u64
/// compressed_size     u32
/// uncompressed_size   u32
/// magic               0xBAADFOOD
/// ------------------------
/// ```
named!(fo4_general_files_parser<&[u8], Vec<BA2File>>,
    many1!(complete!(
        add_return_error!(ErrorKind::Custom(301),
            do_parse!(
                _name_hash:                             le_u32 >>
                _extension:                           take!(4) >>
                _dir_hash:                              le_u32 >>
                _unknown_flags:                         le_u32 >>
                content_offset:                         le_u64 >>
                compressed_size:                        le_u32 >>
                uncompressed_size:                      le_u32 >>
                _magic:  bits!(tag_bits!(u32, 32, 0x0DF0_ADBA)) >>
                (
                    BA2File {
                        header: None,
                        chunks: {
                            let mut chunks = Vec::with_capacity(1);
                            chunks.push(BA2FileChunk {
                                content_offset: content_offset as usize,
                                compressed_size,
                                uncompressed_size,
                            });
                            chunks
                        }
                    }
                )
            )
        )
    ))
);

/// Parses header information about one texture from a texture .ba2 file
///
/// Encoded format
/// ```
/// ------------------------
/// name_hash           u32
/// extension           char[4]
/// dir_hash            u32
/// unknown             u8
/// num_chunks          u8
/// chunk_header_size   u16
/// height              u16
/// width               u16
/// num_mipmaps         u8
/// dxgi_format         u8
/// unknown_2           u16
/// ------------------------
/// ```
named!(fo4_texture_header_parser<&[u8], BA2TextureHeader>,
    add_return_error!(ErrorKind::Custom(302),
        do_parse!(
            _name_hash:         le_u32 >>
            _extension:       take!(4) >>
            _dir_hash:          le_u32 >>
            _unknown:           le_u8  >>
            num_chunks:         le_u8  >>
            chunk_header_size:  le_u16 >>
            height:             le_u16 >>
            width:              le_u16 >>
            num_mipmaps:        le_u8  >>
            dxgi_format:        le_u8  >>
            _unknown_2:         le_u16 >>
            (
                BA2TextureHeader {
                    num_chunks: num_chunks as usize,
                    chunk_header_size,
                    height,
                    width,
                    num_mipmaps,
                    dxgi_format
                }
            )
        )
    )
);

/// Parses all chunks for a single texture file
///
/// Encoded format for a single texture chunk
/// ```
/// ------------------------
/// content_offset      u64
/// compressed_size     u32
/// uncompressed_size   u32
/// mipmap_start        u16
/// mipmap_end          u16
/// magic               0xBAADFOOD
/// ------------------------
/// ```
named!(fo4_texture_chunks_parser<&[u8], Vec<BA2FileChunk>>,
    many1!(complete!(
        add_return_error!(ErrorKind::Custom(303),
            do_parse!(
                content_offset:                         le_u64 >>
                compressed_size:                        le_u32 >>
                uncompressed_size:                      le_u32 >>
                _mipmap_start:                          le_u16 >>
                _mipmap_end:                            le_u16 >>
                _magic: bits!(tag_bits!(u32, 32, 0x0DF0_ADBA)) >>
                (
                    BA2FileChunk {
                        content_offset: content_offset as usize,
                        compressed_size,
                        uncompressed_size,
                    }
                )
            )
        )
    ))
);
