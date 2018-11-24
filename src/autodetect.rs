use std::path::PathBuf;
#[cfg(windows)]
use std::path::Path;

#[cfg(windows)]
use failure::ResultExt;
#[cfg(windows)]
use winreg::{enums::HKEY_LOCAL_MACHINE, RegKey};

use crate::Result;

/// Attempts to detect where a game is installed by querying the Windows registry
#[cfg(windows)]
pub fn autodetect_data_path(game: &str) -> Result<PathBuf> {
    let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
    let subkey_root = Path::new("SOFTWARE\\WOW6432Node\\Bethesda Softworks");
    let subkey = match game {
        "fallout4" => Path::new("Fallout4"),
        "falloutnv" => Path::new("falloutnv"),
        "oblivion" => Path::new("oblivion"),
        "skyrim" => Path::new("skyrim"),
        "skyrimse" => Path::new("Skyrim Special Edition"),
        _ => unimplemented!("Autodetect not supported for this game"),
    };
    let regkey = hklm
        .open_subkey(subkey_root.join(subkey))
        .context(format!("Registry key for {:#?}", subkey))?;
    let installed_path_str: String = regkey.get_value("installed path").context("'installed path' subkey")?;
    Ok(Path::new(&installed_path_str).join("Data"))
}

#[cfg(not(windows))]
pub fn autodetect_data_path(_game: &str) -> Result<PathBuf> {
    Err(format_err!("Data path autodetection is not supported for your platform"))
}
