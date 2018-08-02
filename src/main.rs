extern crate failure;

#[macro_use]
extern crate clap;

extern crate testract;

use std::ffi::OsStr;
use std::path::{Path, PathBuf};

use clap::{App, Arg, ArgGroup, ArgMatches};
use failure::ResultExt;

use testract::autodetect::*;
use testract::{ba2, bsa, ExtensionSet, Result};

fn parse_archives(matches: &ArgMatches, data_path: &PathBuf, output_dir: &Path) -> Result<()> {
    let extension_set = if matches.is_present("all") {
        ExtensionSet::All
    } else if matches.is_present("extensions") {
        ExtensionSet::List(matches.values_of("extensions").unwrap().collect())
    } else {
        ExtensionSet::None
    };

    for dir_entry in data_path.read_dir()? {
        let file_path = dir_entry?.path();
        match file_path.extension().and_then(OsStr::to_str) {
            Some("bsa") => {
                println!("Parsing {:#?}", file_path);
                let bsa_file = bsa::from_file(file_path)?;
                if matches.is_present("header") {
                    println!("{:#?}", bsa_file.header);
                }
                bsa_file.extract_by_extension(&extension_set, output_dir)?
            }
            Some("ba2") => {
                println!("Parsing {:#?}", file_path);
                let ba2_file = ba2::from_file(file_path)?;
                if matches.is_present("header") {
                    println!("{:#?}", ba2_file.header);
                }
                ba2_file.extract_by_extension(&extension_set, output_dir)?
            }
            _ => (),
        };
    }
    Ok(())
}

fn run() -> Result<()> {
    let matches = App::new(crate_name!())
        .version(crate_version!())
        .author(crate_authors!("\n"))
        .about(crate_description!())
        .arg(
            Arg::from_usage("-g, --game [GAME] 'The game to autodetect files for'")
                .possible_values(&["fallout4", "falloutnv", "oblivion", "skyrim", "skyrimse"])
                .case_insensitive(true),
        ).arg(
            Arg::from_usage("-d, --directory [PATH] 'Path to search for files in (not recursive)'").long_help(
                "'Data folder path (e.g. \'C:\\Program Files (x86)\\Steam\\steamapps\\common\\Skyrim\\Data\')",
            ),
        ).group(
            ArgGroup::with_name("choice")
                .args(&["game", "directory"])
                .required(true),
        ).arg(Arg::from_usage(
            "-h, --header 'The header of each BSA file will be printed'",
        )).arg(
            Arg::from_usage("-e, --extensions [EXT] 'A list of file extensions to find (e.g. \'-e png,nif,wav\')'")
                .use_delimiter(true)
                .multiple(true),
        ).arg(Arg::from_usage("-a, --all 'Find all file extensions'"))
        .group(ArgGroup::with_name("find").args(&["extensions", "all"]))
        .arg(
            Arg::from_usage(
                "-o, --output [PATH] 'Folder to output files to (use -o=\'\' or -o\"\" for current directory)'",
            ).requires("find"),
        ).get_matches();

    let data_path = if matches.is_present("game") {
        let game = value_t_or_exit!(matches.value_of("game"), String);
        autodetect_data_path(&game).context(format!("Unable to detect the data path for {}", game))?
    } else {
        let directory = value_t_or_exit!(matches.value_of("directory"), String);
        PathBuf::from(directory)
    };

    let output_dir = if matches.is_present("output") {
        Path::new(matches.value_of("output").unwrap())
    } else {
        Path::new("")
    };

    parse_archives(&matches, &data_path, &output_dir)?;

    println!("All done. Thanks for using testract!");

    Ok(())
}

fn main() {
    if let Err(e) = run() {
        eprint!("error: {}", e);
        let mut e = e.cause();
        while let Some(cause) = e.cause() {
            eprint!(", {}", cause);
            e = cause;
        }
        eprintln!("");
        std::process::exit(1);
    }
}
