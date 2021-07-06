//! Creates ELF files containing data from other files.

use std::fs::File;
use std::io::Error;
use std::io::ErrorKind::InvalidInput;
use std::path::PathBuf;
use std::str::FromStr;
use structopt::StructOpt;

fn main() -> Result<(), Error> {
    let args = CommandLine::from_args();

    let of = File::create(args.out)?;
    let mut builder = elfbin::Builder::new(
        elfbin::Header {
            class: args.class,
            encoding: args.encoding,
            machine: args.machine,
            flags: args.flags,
        },
        of,
    )?;

    for sym_def in args.symbols {
        let name = sym_def.symbol_name;
        let filename = sym_def.filename;
        let f = File::open(filename)?;
        builder.add_symbol(name, f)?;
    }

    let of = builder.close()?;
    of.sync_all()?;

    Ok(())
}

#[derive(StructOpt, Debug, Clone)]
pub struct CommandLine {
    #[structopt(long, name = "class", help = "ELF Class", parse(try_from_str=parse_class), default_value="ELF64")]
    pub class: elfbin::Class,

    #[structopt(long, name = "encoding", help = "ELF Encoding", parse(try_from_str=parse_encoding), default_value="LSB")]
    pub encoding: elfbin::Encoding,

    #[structopt(long, name = "machine", help = "Target machine", parse(try_from_str=parse_machine), default_value="none" )]
    pub machine: u16,

    #[structopt(long, name = "flags", help = "Machine-specific ELF flags", parse(try_from_str=parse_flags), default_value="0x00000000" )]
    pub flags: u32,

    #[structopt(name = "NAME=FILE", help = "Define a symbol")]
    pub symbols: Vec<SymbolDef>,

    #[structopt(short, name = "out", help = "Output filename", required = true)]
    pub out: PathBuf,
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

fn parse_class(src: &str) -> Result<elfbin::Class, Error> {
    match src {
        "elf32" => Ok(elfbin::Class::ELF32),
        "ELF32" => Ok(elfbin::Class::ELF32),
        "elf64" => Ok(elfbin::Class::ELF64),
        "ELF64" => Ok(elfbin::Class::ELF64),
        _ => Err(Error::new(
            std::io::ErrorKind::InvalidInput,
            "class must be either ELF32 or ELF64",
        )),
    }
}

fn parse_encoding(src: &str) -> Result<elfbin::Encoding, Error> {
    match src {
        "LSB" => Ok(elfbin::Encoding::LSB),
        "lsb" => Ok(elfbin::Encoding::LSB),
        "LE" => Ok(elfbin::Encoding::LSB),
        "le" => Ok(elfbin::Encoding::LSB),
        "MSB" => Ok(elfbin::Encoding::MSB),
        "msb" => Ok(elfbin::Encoding::MSB),
        "BE" => Ok(elfbin::Encoding::MSB),
        "be" => Ok(elfbin::Encoding::MSB),
        _ => Err(Error::new(
            std::io::ErrorKind::InvalidInput,
            "class must be either ELF32 or ELF64",
        )),
    }
}

fn parse_machine(src: &str) -> Result<u16, Error> {
    match src {
        "none" => Ok(0),
        "386" => Ok(3),
        "68k" => Ok(4),
        "aarch64" => Ok(183),
        "amd64" => Ok(62),
        "arm" => Ok(40),
        "avr" => Ok(83),
        "riscv" => Ok(243),
        "x64" => Ok(62),
        "x86" => Ok(3),
        "x86_64" => Ok(62),
        _ => {
            if let Some(digits) = src.strip_prefix("0x") {
                match u16::from_str_radix(digits, 16) {
                    Ok(v) => Ok(v),
                    Err(_) => Err(Error::new(
                        InvalidInput,
                        "0x must be followed by up to four hex digits representing an ELF machine id",
                    ))
                }
            } else {
                Err(Error::new(
                    InvalidInput,
                    "machine must either be a hex value (with 0x) prefix, or an architecture keyword",
                ))
            }
        }
    }
}

fn parse_flags(src: &str) -> Result<u32, Error> {
    if let Some(digits) = src.strip_prefix("0x") {
        match u32::from_str_radix(digits, 16) {
            Ok(v) => Ok(v),
            Err(_) => Err(Error::new(
                InvalidInput,
                "0x must be followed by up to eight hex digits representing ELF flags",
            )),
        }
    } else {
        Err(Error::new(
            InvalidInput,
            "flags must be a hex value with 0x prefix",
        ))
    }
}
