use clap::ArgMatches;
use std::{error::Error, fs};
use termcolor::{Color, ColorSpec};

mod argument;
pub use argument::{Argument, Value};

mod instruction;
pub use instruction::Instr;

mod fio;
use fio::{determine_file_type, FileType};

mod ksm_reader;
pub use ksm_reader::{
    ArgumentSection, CodeSection, DebugEntry, DebugSection, KSMFile, KSMFileReader, SectionType,
};

mod coloredout;
pub use coloredout::Terminal;

mod ko_output;
pub use ko_output::{KOFileDebug};

use kerbalobjects::{KOFileReader, KOFile};

pub static NO_COLOR: Color = Color::Rgb(255, 255, 255);

pub static VERSION: &'static str = "1.2.8";

pub static ORANGE_COLOR: Color = Color::Rgb(201, 155, 87);
pub static PURPLE_COLOR: Color = Color::Rgb(133, 80, 179);
pub static DARK_RED_COLOR: Color = Color::Rgb(201, 87, 87);
pub static LIGHT_RED_COLOR: Color = Color::Rgb(255, 147, 147);
pub static GREEN_COLOR: Color = Color::Rgb(129, 181, 154);

pub fn run(config: &CLIConfig) -> Result<(), Box<dyn Error>> {
    // Create the default colorspec as no color
    let no_color = ColorSpec::new();

    // Create a new "Terminal" output object
    let mut term = Terminal::new(no_color);

    term.writeln(&format!("kDump version {}", VERSION))?;

    let filename = config.file_path.to_string();
    let raw_contents = fs::read(filename)?;

    let file_type = determine_file_type(&raw_contents)?;

    match file_type {
        // If this is a compiled kerbal machine code file
        FileType::KSM => {
            let mut ksm_reader = KSMFileReader::new(raw_contents)?;

            let ksm_file = KSMFile::read(&mut ksm_reader)?;

            ksm_file.dump(&config)?;

            Ok(())
        }
        // If it is a kerbal object file
        FileType::KO => {

            let mut ko_reader = KOFileReader::new(raw_contents)?;

            let ko_file =  KOFile::read(&mut ko_reader)?;

            let mut ko_debug = KOFileDebug::new(ko_file);

            ko_debug.dump(&config)?;

            Ok(())
        }
        // If we have no idea what the heck the file is
        FileType::UNKNOWN => {
            return Err("File type not recognized.".into());
        }
    }
}

/// This structure controls all the settings that make this program perform differently
/// These represent command line arguments read in by clap
pub struct CLIConfig {
    pub file_path: String,
    pub disassemble: bool,
    pub disassemble_symbol: bool,
    pub disassemble_symbol_value: String,
    pub file_headers: bool,
    pub argument_section: bool,
    pub line_numbers: bool,
    pub section_headers: bool,
    pub data: bool,
    pub full_contents: bool,
    pub stabs: bool,
    pub syms: bool,
    pub all_headers: bool,
    pub info: bool,
    pub demangle: bool,
    pub show_no_raw_insn: bool,
    pub show_no_labels: bool,
}

impl CLIConfig {
    /// Creates a new CLIConfig using the matches output of clap
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
            data: matches.is_present("data"),
            full_contents: matches.is_present("full_contents"),
            stabs: matches.is_present("stabs"),
            syms: matches.is_present("syms"),
            all_headers: matches.is_present("all_headers"),
            info: matches.is_present("info"),
            demangle: matches.is_present("demangle"),
            show_no_raw_insn: matches.is_present("show_no_raw_insn"),
            show_no_labels: matches.is_present("show_no_labels"),
        }
    }
}
