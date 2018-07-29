use std::path::PathBuf;

mod fallout4;
mod types;

use reader::TESReader;
use Result;

// re-export only types that can be accessed from the main BSA structure
pub use self::types::BA2;

/// Given a file path to a BSA file, opens and parses the archive into the generic BSA structure
pub fn from_file(path: PathBuf) -> Result<BA2> {
    let mut reader = TESReader::from_file(&path)?;
    fallout4::parse_ba2(path, &mut reader)
}
