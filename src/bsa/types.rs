use nom::{le_u16, le_u32};

use Compression;

/// Metadata for the whole archive
#[derive(Debug)]
pub struct BSAHeader {
    /// A single byte indicating the version of the file-format
    pub version: Version,
    /// A list of archive flags indicating how to interpret following records and data (not used by Morrowind)
    pub archive_flags: ArchiveFlags,
    /// List of flags specifying the type of files containing within the archive (not used by Morrowind)
    pub file_flags: FileFlags,
    /// Count of all files in the archive
    pub file_count: usize,
}

/// Metadata for a single file
#[derive(Debug)]
pub struct BSAFile {
    pub has_name: bool,
    /// Decides whether or not the file is compressed
    pub compression: Compression,
    /// Size of the file data
    pub size: u32,
    /// Offset from file byte zero to the raw file data
    pub offset: u32,
}

/// Flag used to indicate what version of the BSA spec this file conforms to
#[derive(Debug, PartialEq)]
pub enum Version {
    /// Morrowind BSAs don't map to a version, so 0x0 was chosen at random
    MORROWIND,
    /// Oblivion files (0x67)
    OBLIVION,
    /// Fallout 3 | Fallout New Vegas | Skyrim files (0x68)
    SKYRIM,
    /// Skyrim Special Edition files (0x69)
    SKYRIMSE,
}

named!(pub version_parser<Version>, switch!(le_u32,
    0x67 => value!(Version::OBLIVION)  |
    0x68 => value!(Version::SKYRIM)    |
    0x69 => value!(Version::SKYRIMSE)
));

bitflags! {
    /// Flags used to indicate how the BSA file should be parsed and/or interpreted
    pub struct ArchiveFlags: u32 {
        /// Include directory names.
        const INCLUDE_DIR_NAMES         = 0b0000_0001;
        /// Include file names.
        const INCLUDE_FILE_NAMES        = 0b0000_0010;
        /// Compressed Archive. When set, files are compressed by default, but can be optionally uncompressed.
        const COMPRESSED_ARCHIVE        = 0b0000_0100;
        /// Retain directory names
        const RETAIN_DIR_NAMES          = 0b0000_1000;
        /// Retain file names
        const RETAIN_FILE_NAMES         = 0b0001_0000;
        /// Retain file name offsets
        const RETAIN_FILE_NAME_OFFSETS  = 0b0010_0000;
        /// Xbox 360 archive. Hash values and numbers after the header are encoded in big-endian
        const XBOX_360_ARCHIVE          = 0b0100_0000;
        /// Retain strings during startup
        const RETAIN_STARTUP_STRINGS    = 0b1000_0000;
        /// Indicates whether file records begin with a bstring containing the name of file
        const EMBED_FILE_NAMES        = 0b1_0000_0000;
        /// Xbox 360 only compression algorithm (must be used with COMPRESSED_ARCHIVE flag)
        const XMEM_CODEC             = 0b10_0000_0000;
        /// An unknown flag found in Official Oblivion BSA files
        const UNKNOWN_OBLIVION_FLAG = 0b100_0000_0000;
    }
}

named!(
    pub parse_archive_flags<ArchiveFlags>,
    add_return_error!(ErrorKind::Custom(1), map_opt!(le_u32, ArchiveFlags::from_bits))
);

bitflags! {
    /// Flags used to indicate the category of files contained in the archive
    pub struct FileFlags: u16 {
        /// Mesh files (.nif)
        const MESHES    = 0b0000_0001;
        /// Texture files (.dds)
        const TEXTURES  = 0b0000_0010;
        /// Menu files (.xml/.swf)
        const MENUS     = 0b0000_0100;
        /// Music files (.xwm) and sound files (.wav)
        const SOUNDS    = 0b0000_1000;
        /// Voice files (.mp3/.fuz)
        const VOICES    = 0b0001_0000;
        /// Shader files (.fxp/.txt/.html/.bat/.scc)
        const SHADERS   = 0b0010_0000;
        /// Tree files (.spt/.btt/.btr)
        const TREES     = 0b0100_0000;
        /// Font files (.tex/.fnt/.swf)
        const FONTS     = 0b1000_0000;
        /// Miscellaneous files (.gid/.pex)
        const MISC    = 0b1_0000_0000;
    }
}

named!(
    pub parse_file_flags<FileFlags>,
    add_return_error!(ErrorKind::Custom(2), map_opt!(le_u16, FileFlags::from_bits))
);
