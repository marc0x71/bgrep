use std::{fs::File, io::BufReader, path::PathBuf};

use anyhow::{Result, bail};
use clap::Parser;

use crate::{
    binary_search::{binary_search_in_file, print_all_occurrences},
    value::Value,
};

mod binary_search;
mod value;

#[derive(Parser, Debug)]
#[command(name = "bgrep")]
#[command(
    long_about = "Search for KEY in a sorted delimited text file using binary search.\nUnlike grep, bgrep requires the file to be sorted on the key column,\nbut runs in O(log n) time instead of O(n).",
    about = " Search for KEY in a sorted delimited file using binary search (O(log n)).\nFILE must be pre-sorted on the key column."
)]
struct Cli {
    /// Index of the column to use as key
    #[arg(short = 'k', long = "key", default_value_t = 0)]
    key_column: usize,

    /// Delimiter character
    #[arg(short = 'd', long = "delimiter", default_value = ";")]
    delimiter: String,

    /// If set, the key is numeric
    #[arg(short = 'n', long = "numeric", default_value_t = false)]
    numeric: bool,

    /// Value to search for
    #[arg(required = true)]
    target: String,

    /// Path to the input file
    #[arg(required = true)]
    file: PathBuf,
}

fn run() -> Result<bool> {
    let args = Cli::parse();

    if args.delimiter.len() > 1 {
        bail!(
            "Invalid delimiter '{}', must be a single character",
            args.delimiter
        );
    }
    let delimiter = args.delimiter.chars().next().unwrap_or_default();

    if !args.file.exists() {
        bail!("File '{}' not found", args.file.display());
    }

    let file = File::open(args.file)?;
    let mut buffer = BufReader::new(file);

    let target = Value::build(&args.target, args.numeric)?;
    let found = binary_search_in_file(
        &mut buffer,
        delimiter,
        &target,
        args.key_column,
        args.numeric,
    )?;

    let mut match_found = false;

    // Print all occurrences starting from the found position
    if let Some(start_position) = found {
        match_found = true;
        print_all_occurrences(
            &mut buffer,
            &mut std::io::stdout(),
            start_position,
            delimiter,
            &target,
            args.key_column,
            args.numeric,
        )?;
    }

    Ok(match_found)
}

fn main() {
    std::process::exit(match run() {
        Ok(found) => {
            if found {
                0
            } else {
                1
            }
        }
        Err(e) => {
            eprintln!("Error: {e}");
            2
        }
    });
}
