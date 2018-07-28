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
pub mod autodetect;
mod ba2;
mod bsa;
mod reader;

// Re-exports
pub use ba2::BA2;
pub use bsa::BSA;

/// Result alias for wrapping the `failure::Error` type
pub type Result<T> = ::std::result::Result<T, Error>;

type ParserFn<O> = fn(input: &[u8]) -> IResult<&[u8], O>;

#[allow(needless_pass_by_value)]
fn convert_nom_err<P: Debug>(e: Err<P>) -> Error {
    err_msg(format!("Failed to parse: {}", e))
}

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
        use ExtensionSet::*;
        match self {
            None => false,
            All => true,
            List(ext_list) => ext_list.contains(&file_extension),
        }
    }
}
