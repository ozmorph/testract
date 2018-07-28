// Copyright Notice:   The Elder Scrolls, Morrowind, Oblivion, Skyrim, and Fallout are registered trademarks or trademarks of ZeniMax Media Inc.
//
//! TEStract is a utility for extracting infomation from various Bethesda archives: .bsa, .ba2
//!
//! BSA parsing support is currently available for the following games:
//!
//!   * Fallout New Vegas
//!   * Morrowind
//!   * Oblivion
//!   * Skyrim (Original + Legendary Edition)
//!   * Skyrim Special Edition
#![allow(unknown_lints)]
#![deny(missing_docs)]
#![deny(warnings)]

#[macro_use]
extern crate bitflags;

#[macro_use]
extern crate nom;

#[macro_use]
extern crate failure;

#[macro_use]
extern crate clap;

extern crate byteorder;
extern crate flate2;
extern crate lz4;
extern crate twox_hash;

#[cfg(windows)]
extern crate winreg;

use nom::{Err, IResult};
use std::fmt::Debug;

use failure::{err_msg, Error};

// the AutodetectGames enum is unable to be documented because of the arg_enum! macro
#[allow(missing_docs)]
mod autodetect;

mod ba2;
mod bsa;
mod reader;

// Re-exports
pub use autodetect::{autodetect_data_path, AutodetectGames};
pub use ba2::{BA2};
pub use bsa::{ArchiveFlags, BSAFile, BSAHeader, FileFlags, Version, BSA};
pub use reader::TESReader;

/// Result alias for wrapping the `failure::Error` type
pub type Result<T> = ::std::result::Result<T, Error>;

type ParserFn<O> = fn(input: &[u8]) -> IResult<&[u8], O>;

#[allow(needless_pass_by_value)]
fn convert_nom_err<P: Debug>(e: Err<P>) -> Error {
    err_msg(format!("Failed to parse: {}", e))
}
