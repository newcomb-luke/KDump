use clap::ArgMatches;
use colored::*;

use std::{collections::HashMap, error::Error, fs, iter::Peekable, slice::Iter};

mod argument;
pub use argument::{Argument, Value};

mod instruction;
pub use instruction::Instr;

mod fio;
use fio::{determine_file_type, FileType};

mod ksm_reader;
use ksm_reader::{ArgumentSection, KSMFile, KSMFileReader, SectionType};

pub static VERSION: &'static str = "1.0.0";
pub static LINE_COLOR: (u8, u8, u8) = (201, 155, 87);
pub static ADDRESS_COLOR: (u8, u8, u8) = (133, 80, 179);
pub static MNEMONIC_COLOR: (u8, u8, u8) = (201, 87, 87);
pub static VARIABLE_COLOR: (u8, u8, u8) = (255, 147, 147);

pub fn run(config: &CLIConfig) -> Result<(), Box<dyn Error>> {
    println!("kDump version {}", VERSION);

    let filename = config.file_path.to_string();
    let raw_contents = fs::read(filename)?;

    let file_type = determine_file_type(&raw_contents)?;

    match file_type {
        FileType::KSM => {
            let mut ksm_reader = KSMFileReader::new(raw_contents)?;

            let ksm_file = KSMFile::read(&mut ksm_reader)?;

            ksm_file.dump(&config);

            Ok(())
        }
        FileType::KO => {
            return Err("KerbalObject file dumping has not yet been implemented.".into());
        }
        FileType::UNKNOWN => {
            return Err("File type not recognized.".into());
        }
    }
}

pub struct CLIConfig {
    pub file_path: String,
    pub disassemble: bool,
    pub disassemble_symbol: bool,
    pub disassemble_symbol_value: String,
    pub file_headers: bool,
    pub argument_section: bool,
    pub line_numbers: bool,
    pub section_headers: bool,
    pub full_contents: bool,
    pub stabs: bool,
    pub syms: bool,
    pub all_headers: bool,
    pub info: bool,
    pub demangle: bool,
    pub show_no_raw_insn: bool,
    pub show_no_addresses: bool,
}

impl CLIConfig {
    pub fn new(matches: ArgMatches) -> CLIConfig {
        CLIConfig {
            file_path: String::from(matches.value_of("INPUT").unwrap()),
            disassemble: matches.is_present("disassemble"),
            disassemble_symbol: matches.is_present("disassemble_symbol"),
            disassemble_symbol_value: if matches.is_present("disassemble_symbol") {
                String::from(matches.value_of("disassemble_symbol").unwrap())
            } else {
                String::new()
            },
            file_headers: matches.is_present("file_headers"),
            argument_section: matches.is_present("argument_section"),
            line_numbers: matches.is_present("line_numbers"),
            section_headers: matches.is_present("section_headers"),
            full_contents: matches.is_present("full_contents"),
            stabs: matches.is_present("stabs"),
            syms: matches.is_present("syms"),
            all_headers: matches.is_present("all_headers"),
            info: matches.is_present("info"),
            demangle: matches.is_present("demangle"),
            show_no_raw_insn: matches.is_present("show_no_raw_insn"),
            show_no_addresses: matches.is_present("show_no_addresses"),
        }
    }
}