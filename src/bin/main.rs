//! Creates ELF files containing data from other files.

use std::fs::File;
use std::io::Error;
use std::path::PathBuf;
use std::str::FromStr;
use structopt::StructOpt;

#[derive(StructOpt, Debug, Clone)]
pub struct CommandLine {
    #[structopt(name = "NAME=FILE", help = "Define a symbol", required = true)]
    pub symbols: Vec<SymbolDef>,
}

#[derive(Debug, Clone)]
pub struct SymbolDef {
    pub symbol_name: String,
    pub filename: PathBuf,
}

impl FromStr for SymbolDef {
    type Err = Error;

    fn from_str(from: &str) -> Result<Self, Error> {
        match from.split_once('=') {
            None => Err(Error::new(
                std::io::ErrorKind::InvalidInput,
                "symbol definition must be NAME=FILENAME",
            )),
            Some((symname, filename)) => Ok(Self {
                symbol_name: String::from(symname),
                filename: PathBuf::from(filename),
            }),
        }
    }
}

fn main() -> Result<(), Error> {
    let args = CommandLine::from_args();

    println!("args {:#?}", args);
    Ok(())
}
