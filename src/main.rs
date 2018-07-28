#[macro_use]
extern crate failure;

#[macro_use]
extern crate clap;

extern crate testract;

use std::ffi::OsStr;
use std::fs;
use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};

use clap::{App, Arg, ArgGroup, ArgMatches};
use failure::ResultExt;

use testract::{autodetect_data_path, AutodetectGames, BSAFile, Result, TESReader, BSA};

fn parse_bsas(matches: &ArgMatches, data_path: &PathBuf) -> Result<Vec<BSA>> {
    let mut bsa_files: Vec<BSA> = Vec::new();
    for dir_entry in data_path.read_dir()? {
        let dir_entry = dir_entry?;
        let file_path = dir_entry.path();
        let is_bsa = |file_path: &PathBuf| match file_path.extension() {
            Some(extension) => extension == "bsa" || extension == "ba2",
            None => false,
        };
        if is_bsa(&file_path) {
            println!("Parsing {:#?}", file_path);
            let bsa_file = BSA::from_file(file_path)?;
            if matches.is_present("header") {
                println!("{:#?}", bsa_file.header);
            }
            bsa_files.push(bsa_file);
        }
    }
    Ok(bsa_files)
}

fn dump_file(output_dir: &Path, file_name: &Path, file: &BSAFile, bsa_file: &BSA, bsa_path: &Path) -> Result<()> {
    let file_path = output_dir.join(file_name);
    let mut reader = TESReader::from_file(bsa_path)?;
    let data = bsa_file.extract_via_file(&mut reader, file)?;
    fs::create_dir_all(file_path
        .parent()
        .ok_or_else(|| format_err!("{:#?} has no parent dir", file_path))?)?;
    let mut file_handle = File::create(&file_path)?;
    file_handle.write_all(&data)?;
    Ok(())
}

fn run() -> Result<()> {
    let matches = App::new(crate_name!())
        .version(crate_version!())
        .author(crate_authors!("\n"))
        .about(crate_description!())
        .arg(
            Arg::from_usage("-g, --game [GAME] 'The game to autodetect files for'")
                .possible_values(&AutodetectGames::variants())
                .case_insensitive(true),
        )
        .arg(
            Arg::from_usage("-d, --directory [PATH] 'Path to search for files in (not recursive)'").long_help(
                "'Data folder path (e.g. \'C:\\Program Files (x86)\\Steam\\steamapps\\common\\Skyrim\\Data\')",
            ),
        )
        .group(
            ArgGroup::with_name("choice")
                .args(&["game", "directory"])
                .required(true),
        )
        .arg(Arg::from_usage(
            "-h, --header 'The header of each BSA file will be printed'",
        ))
        .arg(
            Arg::from_usage("-e, --extension [EXT] 'A list of file extensions to find (e.g. \'-e png,nif,wav\')'")
                .use_delimiter(true)
                .multiple(true),
        )
        .arg(Arg::from_usage("-a, --all 'Find all file extensions'"))
        .group(ArgGroup::with_name("find").args(&["extension", "all"]))
        .arg(
            Arg::from_usage(
                "-o, --output [PATH] 'Folder to output files to (use -o=\'\' or -o\"\" for current directory'",
            ).requires("find"),
        )
        .get_matches();

    let data_path = if matches.is_present("game") {
        let game = value_t_or_exit!(matches.value_of("game"), AutodetectGames);
        autodetect_data_path(&game).context(format!("Unable to detect the data path for {:#?}", game))?
    } else {
        let directory = value_t_or_exit!(matches.value_of("directory"), String);
        PathBuf::from(directory)
    };

    let output_dir = if matches.is_present("output") {
        Path::new(matches.value_of("output").unwrap())
    } else {
        Path::new("")
    };

    let bsa_files = parse_bsas(&matches, &data_path)?;

    // we only iterate over the files in the bsas if the user requested them
    let all_flag = matches.is_present("all");
    if all_flag || matches.is_present("extension") {
        let extensions: Vec<&str> = matches.values_of("extension").unwrap().collect();
        for bsa_file in &bsa_files {
            for (file_name, file) in &bsa_file.file_hashmap {
                match file_name.extension().and_then(OsStr::to_str) {
                    Some(extension) if all_flag || extensions.contains(&extension) => {
                        println!("{:#?}", file_name);
                        if matches.is_present("output") {
                            dump_file(&output_dir, &file_name, &file, &bsa_file, &bsa_file.path)?
                        }
                    }
                    _ => (),
                }
            }
        }
    }

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
