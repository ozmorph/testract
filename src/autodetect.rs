use std::path::{Path, PathBuf};

use failure::ResultExt;
use winreg::{enums::HKEY_LOCAL_MACHINE, RegKey};

use Result;

arg_enum!{
    /// Game variants that are supported by testract for autodetection of the folder
    #[derive(Debug)]
    pub enum AutodetectGames {
        FALLOUT4,
        FALLOUTNV,
        OBLIVION,
        SKYRIM,
        SKYRIMSE,
    }
}

/// Attempts to detect where a game is installed by querying the Windows registry
#[cfg(windows)]
pub fn autodetect_data_path(game: &AutodetectGames) -> Result<PathBuf> {
    let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
    let subkey_root = Path::new("SOFTWARE\\WOW6432Node\\Bethesda Softworks");
    let subkey = match game {
        AutodetectGames::FALLOUT4 => Path::new("Fallout4"),
        AutodetectGames::FALLOUTNV => Path::new("falloutnv"),
        AutodetectGames::OBLIVION => Path::new("oblivion"),
        AutodetectGames::SKYRIM => Path::new("skyrim"),
        AutodetectGames::SKYRIMSE => Path::new("Skyrim Special Edition"),
    };
    let regkey = hklm.open_subkey(subkey_root.join(subkey))
        .context(format!("Registry key for {:#?}", subkey))?;
    let installed_path_str: String = regkey.get_value("installed path").context("'installed path' subkey")?;
    Ok(Path::new(&installed_path_str).join("Data"))
}

#[cfg(not(windows))]
pub fn autodetect_data_path(game: &Game) -> Result<PathBuf> {
    Err(format_err!(
        "Data path autodetection is not supported for your platform"
    ));
}