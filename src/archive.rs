use std::collections::HashMap;
use std::ffi::OsStr;
use std::hash::BuildHasherDefault;
use std::path::{Path, PathBuf};

use twox_hash::XxHash;

use reader::{TESFile, TESReader};
use {dump_to_file, Result};

pub type FileMap<F> = HashMap<PathBuf, F, BuildHasherDefault<XxHash>>;

/// List of file extensions
#[derive(PartialEq)]
pub enum ExtensionSet<'a> {
    /// Represents an empty list
    None,
    /// A list of one or more extensions
    List(Vec<&'a str>),
    /// The set of all possible extensions
    All,
}

impl<'a> ExtensionSet<'a> {
    /// Determines if a given file extension has a matches within the set
    pub fn is_match(&self, file_extension: &str) -> bool {
        use archive::ExtensionSet::*;
        match self {
            None => false,
            All => true,
            List(ext_list) => ext_list.contains(&file_extension),
        }
    }
}

pub struct Archive<H, F> {
    /// Path on disk to this file
    pub path: PathBuf,
    /// Header containing metadata for the entire archive
    pub header: H,
    /// HashMap mapping file paths to files
    pub file_hashmap: FileMap<F>,
}

impl<H, F: Extract> Archive<H, F> {
    /// Given a set of extensions, find all of the files that match it
    fn get_by_extension(&self, extension_set: &ExtensionSet) -> Vec<&Path> {
        let mut file_names = Vec::new();

        if *extension_set == ExtensionSet::None {
            return file_names;
        }

        for file_name in self.file_hashmap.keys() {
            if *extension_set != ExtensionSet::All {
                if let Some(extension) = file_name.extension().and_then(OsStr::to_str) {
                    if !extension_set.is_match(&extension) {
                        continue;
                    }
                }
            }

            println!("{:#?}", file_name);
            file_names.push(file_name);
        }
        file_names
    }

    /// Given a set of extensions
    pub fn extract_by_extension(&self, extension_set: &ExtensionSet, output_dir: &Path) -> Result<()> {
        let file_names = self.get_by_extension(&extension_set);
        if output_dir != Path::new("") && !file_names.is_empty() {
            let mut reader = TESReader::from_file(&self.path)?;
            for file_name in file_names {
                let file_data = self.extract_by_name(&mut reader, file_name)?;
                dump_to_file(&output_dir, &file_name, &file_data)?
            }
        }
        Ok(())
    }

    /// Given a file path, extracts the file content from the BSA
    pub fn extract_by_name(&self, reader: &mut TESFile, file_path: &Path) -> Result<Vec<u8>> {
        let file_record = self
            .file_hashmap
            .get(file_path)
            .ok_or_else(|| format_err!("File {:#?} not found", file_path))?;
        file_record.extract(reader)
    }
}

pub trait Extract {
    fn extract(&self, reader: &mut TESFile) -> Result<Vec<u8>>;
}
