use std::path::PathBuf;

mod fallout4;
mod types;

use archive::{Archive, Extract};
use reader::{TESFile, TESReader};
use Result;

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
    fn extract(&self, _reader: &mut TESFile) -> Result<Vec<u8>> {
        unimplemented!("Currently unimplemented");
    }
}
