// Copyright Notice:   The Elder Scrolls, Morrowind, Oblivion, Skyrim, and Fallout are registered trademarks or
// trademarks of ZeniMax Media Inc.
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
// #![deny(missing_docs)]
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

use std::fmt::Debug;
use std::fs;
use std::fs::File;
use std::io::{Read, Write};
use std::path::Path;

use byteorder::{ByteOrder, LittleEndian};
use failure::{err_msg, Error, ResultExt};
use flate2::read::ZlibDecoder;
use nom::{Err, IResult};

// the AutodetectGames enum is unable to be documented because of the arg_enum! macro
#[allow(missing_docs)]
pub mod autodetect;

mod archive;
pub mod ba2;
pub mod bsa;
mod reader;

// Re-exports
pub use archive::ExtensionSet;

/// Result alias for wrapping the `failure::Error` type
pub type Result<T> = ::std::result::Result<T, Error>;

type ParserFn<O> = fn(input: &[u8]) -> IResult<&[u8], O>;

#[allow(needless_pass_by_value)]
fn convert_nom_err<P: Debug>(e: Err<P>) -> Error {
    err_msg(format!("Failed to parse: {}", e))
}

/// Dumps a slice of bytes to the file path made by combining output_dir and file_name
fn dump_to_file(output_dir: &Path, file_name: &Path, file_data: &[u8]) -> Result<()> {
    let file_path = output_dir.join(file_name);
    fs::create_dir_all(
        file_path
            .parent()
            .ok_or_else(|| format_err!("{:#?} has no parent dir", file_path))?,
    )?;
    let mut file_handle = File::create(&file_path)?;
    file_handle.write_all(file_data)?;
    Ok(())
}

#[derive(Debug, PartialEq)]
pub enum Compression {
    None,
    Zlib,
    Lz4,
}

impl Compression {
    fn decompress_buffer(&self, buffer: &[u8]) -> Result<Vec<u8>> {
        let (length, data) = buffer.split_at(4);
        let uncompressed_length = LittleEndian::read_u32(length);
        let mut out_buffer = Vec::with_capacity(uncompressed_length as usize);
        match self {
            Compression::Zlib => {
                let mut decoder = ZlibDecoder::new(data);
                decoder
                    .read_to_end(&mut out_buffer)
                    .context("Unable to decompress ZLIB data")?;
            }
            Compression::Lz4 => {
                let mut decoder = lz4::Decoder::new(data)?;
                decoder
                    .read_to_end(&mut out_buffer)
                    .context("Unable to decompress LZ4 data")?;
            }
            Compression::None => out_buffer = data.to_vec(),
        };
        Ok(out_buffer)
    }
}
