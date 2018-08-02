//! Module for opening and decoding Bethesda file formats.
//!
//! Some of the code in this module was inspired from two projects:
//! The [Reader](https://github.com/tafia/quick-xml/blob/master/src/reader.rs) struct in [quick-xml](https://crates.io/search?q=quick-xml).
//! The [CborReader](https://github.com/BurntSushi/rust-cbor/blob/master/src/decoder.rs) struct in [rust-cbor](https://crates.io/crates/cbor).
use std::fs::File;
use std::io;
use std::io::{BufRead, BufReader, Read, Seek, SeekFrom};
use std::path::Path;

use byteorder::{ByteOrder, LittleEndian};
use failure::ResultExt;

use {convert_nom_err, ParserFn, Result};

/// The files contain ISO-8859-1 encoded strings. This function attempts to create a UTF8 string by mapping each
/// individual byte to a char primitive which are always interpreted by Rust as UTF8 (up to 4 bytes). As a result,
/// each ISO-8859-1 character will automatically be converted to its UTF8 equivalent and can then be collected at
/// the end into a single String type.
///
/// Credit: <https://stackoverflow.com/questions/28169745/what-are-the-options-to-convert-iso-8859-1-latin-1-to-a-string-utf-8/28175593#28175593>
pub fn latin1_to_string(buffer: &[u8]) -> String {
    buffer.iter().map(|&c| c as char).collect()
}

/// Thin wrapper over a buffered reader providing functionality specific to parsing TES files
pub struct TESReader<B: BufRead> {
    /// Underlying buffered reader
    pub reader: B,
}

/// Type alias for reading from a file
pub type TESFile = TESReader<BufReader<File>>;

impl TESFile {
    /// Creates a TESReader from a file path.
    pub fn from_file<P: AsRef<Path>>(path: P) -> io::Result<TESFile> {
        let file = File::open(path)?;
        let reader = BufReader::new(file);
        Ok(TESReader::from_reader(reader))
    }
}

impl<B: BufRead + Seek> TESReader<B> {
    /// Opens a buffered file reader at location `file_name` and returns it as a TESFileReader
    pub fn from_reader(reader: B) -> Self {
        Self { reader }
    }

    /// Reads a string with a single byte prefixed for length from the file at the current seek position.
    fn read_string_with_len_prefix(&mut self) -> io::Result<Vec<u8>> {
        // Read the length first to know how big of a vector to allocate
        let length = self.parse_byte()?;

        // Allocate the byte buffer for the full string and read it in
        let mut buffer: Vec<u8> = vec![0; length as usize];
        self.read_exact(&mut buffer)?;

        Ok(buffer)
    }

    fn read_string_with_dlen_prefix(&mut self) -> io::Result<Vec<u8>> {
        // The length of the string is denoted by two bytes
        let length = self.parse_short()?;

        // Allocate the byte buffer for the full string and read it in
        let mut buffer: Vec<u8> = vec![0; length as usize];
        self.read_exact(&mut buffer)?;

        Ok(buffer)
    }

    /// Reads a block of '\0' terminated latin-1 strings and parses them into a vector of UTF8 strings  
    pub fn parse_bstring_block(&mut self, total_length: usize) -> Result<Vec<String>> {
        // Read a bstring block
        let mut buffer = vec![0; total_length];
        self.read_exact(&mut buffer)?;

        // convert the buffer to a UTF8 string
        let bstring_block = latin1_to_string(&buffer);

        // Split the UTF8 string into a vector of '\0' terminated strings
        let mut bstrings: Vec<String> = bstring_block.split_terminator('\0').map(|s| s.to_string()).collect();
        bstrings.shrink_to_fit();

        Ok(bstrings)
    }

    /// Reads a precise number of bytes and applies a named Nom parser function to it.
    pub fn parse_exact<O>(&mut self, input_size: usize, parse_func: ParserFn<O>) -> Result<O> {
        let mut input_buffer = vec![0; input_size];
        self.read_exact(&mut input_buffer)
            .context(format!("Failed to read {} bytes", input_size))?;
        let (_, output_type) = parse_func(&input_buffer).map_err(convert_nom_err)?;
        Ok(output_type)
    }

    /// Parses a byte from the underlying buffer.
    fn parse_byte(&mut self) -> io::Result<u8> {
        let mut buffer = [0; 1];
        self.read_exact(&mut buffer)?;
        Ok(buffer[0])
    }

    /// Parses a short from the underlying buffer.
    fn parse_short(&mut self) -> io::Result<u16> {
        let mut buffer = [0; 2];
        self.read_exact(&mut buffer)?;
        Ok(LittleEndian::read_u16(&buffer))
    }

    /// Reads bytes until a '\0' is encountered.
    pub fn parse_zstring(&mut self) -> io::Result<String> {
        let mut string_buf = Vec::new();
        self.read_until(b'\0', &mut string_buf)?;
        // When Rust creates a String object, it always appends a '\0'; so we only convert the first n-1 bytes
        Ok(latin1_to_string(&string_buf[0..string_buf.len() - 1]))
    }

    /// Reads a string prefixed with a byte length. NOT zero terminated.
    pub fn parse_bstring(&mut self) -> io::Result<String> {
        let string_buf = self.read_string_with_len_prefix()?;
        Ok(latin1_to_string(&string_buf))
    }

    /// Reads a string prefixed with a short length. NOT zero terminated.
    pub fn parse_long_bstring(&mut self) -> io::Result<String> {
        let string_buf = self.read_string_with_dlen_prefix()?;
        Ok(latin1_to_string(&string_buf))
    }

    /// Reads a string prefixed with a byte length and terminated with a zero '\0'.
    pub fn parse_bzstring(&mut self) -> io::Result<String> {
        let string_buf = self.read_string_with_len_prefix()?;
        // When Rust creates a String object, it always appends a '\0'; so we only convert the first n-1 bytes
        Ok(latin1_to_string(&string_buf[0..string_buf.len() - 1]))
    }
}

impl<S: BufRead + Seek> Seek for TESReader<S> {
    fn seek(&mut self, pos: SeekFrom) -> io::Result<u64> {
        self.reader.seek(pos)
    }
}

impl<B: BufRead> Read for TESReader<B> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        Ok(self.reader.read(buf)?)
    }
}

impl<B: BufRead> BufRead for TESReader<B> {
    fn fill_buf(&mut self) -> io::Result<&[u8]> {
        Ok(self.reader.fill_buf()?)
    }

    fn consume(&mut self, amt: usize) {
        self.reader.consume(amt)
    }
}
