// SPDX-FileCopyrightText: 2025 Marek Rusinowski
// SPDX-License-Identifier: Apache-2.0
use anyhow::Result;
use clap::{crate_name, crate_version, Parser};
use clio::{Input, Output};
use regex::Regex;
use serde::Serialize;
use std::io::{BufRead, BufReader, BufWriter};
use std::os::linux::fs::MetadataExt;
use std::path::{Path, PathBuf};
use std::time::SystemTime;
use struson::writer::{JsonStreamWriter, JsonWriter};

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Input with log from duplicacy
    #[arg(short, long, default_value = "-")]
    input: Input,

    /// Output to write NCDU Json Export
    #[arg(short, long, default_value = "-")]
    output: Output,
}

#[derive(Serialize)]
struct FileInfo<'a> {
    name: &'a str,
    asize: u64,
    dsize: u64,
    dev: u64,
    ino: u64,
    nlink: u64,
    notreg: bool,
}

#[derive(Serialize)]
struct NcduMetadata {
    progname: &'static str,
    progver: &'static str,
    timestamp: u64,
}

fn write_infoblock<J: JsonWriter>(json_writer: &mut J, path: &Path) -> Result<()> {
    let meta = std::fs::symlink_metadata(path)?;
    let name = if path.is_absolute() {
        path.to_str().unwrap()
    } else {
        path.file_name().unwrap().to_str().unwrap()
    };
    json_writer.serialize_value(&FileInfo {
        name,
        asize: meta.st_size(),
        dsize: meta.st_blocks() * 512,
        dev: meta.st_dev(),
        ino: meta.st_ino(),
        nlink: meta.st_nlink(),
        notreg: !meta.is_dir() && !meta.is_file(),
    })?;
    Ok(())
}

fn main() -> Result<()> {
    let args = Args::parse();
    let reader = BufReader::new(args.input);

    // The format of file inclusion lines when duplicacy is run with `-debug -log backup -enum-only`
    let include_re = Regex::new(r"^\d{4}-\d{2}-\d{2} \d{2}:\d{2}:\d{2}.\d{3} DEBUG PATTERN_INCLUDE (.*) is included(?: by pattern .*)?$").unwrap();

    let mut json_writer = JsonStreamWriter::new(BufWriter::new(args.output));

    json_writer.begin_array()?;
    // Format compatible with NCDU >=1.16
    json_writer.number_value(1)?;
    json_writer.number_value(2)?;
    json_writer.serialize_value(&NcduMetadata {
        progname: crate_name!(),
        progver: crate_version!(),
        timestamp: SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs(),
    })?;

    json_writer.begin_array()?;
    write_infoblock(&mut json_writer, std::env::current_dir()?.as_path())?;

    // Holds current stack of open directories to open and close corresponding
    // json arrays as we stream through files included by duplicacy. We can do
    // this because Duplicacy visits files in a depth-first-search order.
    let mut dir = PathBuf::new();

    for line_or in reader.lines() {
        let line = line_or?;
        if let Some(caps) = include_re.captures(line.as_str()) {
            let (_, [path_str]) = caps.extract();

            // We ignore all directories, we care only about files
            if path_str.ends_with("/") {
                continue;
            }
            let path = Path::new(path_str);

            // Get to the common ancestor of previously handled file and current one.
            while !path.starts_with(dir.as_path()) {
                dir.pop();
                json_writer.end_array()?;
            }

            // Open all directories from common ancestor to the parent of current file.
            for c in path
                .strip_prefix(dir.as_path())
                .unwrap()
                .parent()
                .unwrap()
                .components()
            {
                dir.push(c);
                json_writer.begin_array()?;
                write_infoblock(&mut json_writer, dir.as_path())?;
            }

            // Finally dump information about currently handled file.
            write_infoblock(&mut json_writer, path)?;
        }
    }

    while dir.pop() {
        json_writer.end_array()?;
    }

    json_writer.end_array()?;
    json_writer.end_array()?;
    json_writer.finish_document()?.into_inner()?.finish()?;
    Ok(())
}
