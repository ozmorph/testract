use nom::le_u32;

/// Metadata for the whole archive.
#[derive(Debug)]
pub struct BA2Header {
    /// Version of the file (should always be 0x1)
    pub version: BA2Version,
    /// Type of this BA2 archive
    pub file_type: BA2Type,
    /// Count of all files in the archive
    pub file_count: usize,
    /// Offset to the start of the file names
    pub name_table_offset: u64,
}

#[derive(Debug)]
pub struct BA2TextureHeader {
    /// Number of file chunks
    pub num_chunks: usize,
    /// Size of one chunk header
    pub chunk_header_size: u16,
    /// Height of the texture
    pub height: u16,
    /// Width of the texture
    pub width: u16,
    /// Number of mipmaps
    pub num_mipmaps: u8,
    /// The DXGI encoding format for the texture
    pub dxgi_format: u8,
}

#[derive(Debug)]
pub struct BA2FileChunk {
    /// Offset from the start of the file to this chunk's data
    pub content_offset: u64,
    /// Size of contents while zlib compressed (if 0, then the file isn't compressed)
    pub compressed_size: usize,
    /// Size of contents while uncompresed
    pub uncompressed_size: usize,
}

#[derive(Debug)]
pub struct BA2File {
    pub header: Option<BA2TextureHeader>,
    pub chunks: Vec<BA2FileChunk>,
}

/// The type of files contained in the BA2 archive
#[derive(Debug, PartialEq)]
pub enum BA2Type {
    /// Encoded as "GNRL"
    General,
    /// Encoded as "DX10"
    Textures,
}

named!(pub type_parser<BA2Type>, alt!(
    tag!("GNRL")        => { |_| BA2Type::General  } |
    tag!("DX10")        => { |_| BA2Type::Textures }
));

#[derive(Debug)]
pub enum BA2Version {
    /// Fallout 4 files (0x1)
    Fallout4,
}

named!(pub version_parser<BA2Version>, switch!(le_u32,
    0x1 => value!(BA2Version::Fallout4)
));
