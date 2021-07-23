use clap::ArgMatches;
use kerbalobjects::kofile::KOFile;
use kerbalobjects::{ksmfile::KSMFile, FromBytes};
use std::io::Write;
use std::{error::Error, fs};
use termcolor::{Color, ColorSpec, StandardStream};

mod fio;
use fio::{determine_file_type, FileType};

mod output;
use output::KOFileDebug;
use output::KSMFileDebug;

pub static NO_COLOR: Color = Color::Rgb(255, 255, 255);

pub static VERSION: &'static str = env!("CARGO_PKG_VERSION");

pub static ORANGE_COLOR: Color = Color::Rgb(201, 155, 87);
pub static PURPLE_COLOR: Color = Color::Rgb(133, 80, 179);
pub static DARK_RED_COLOR: Color = Color::Rgb(201, 87, 87);
pub static LIGHT_RED_COLOR: Color = Color::Rgb(255, 147, 147);
pub static GREEN_COLOR: Color = Color::Rgb(129, 181, 154);

pub fn run(config: &CLIConfig) -> Result<(), Box<dyn Error>> {
    let mut stream = StandardStream::stdout(termcolor::ColorChoice::Auto);

    let mut no_color = ColorSpec::new();
    no_color.set_fg(Some(NO_COLOR));

    writeln!(stream, "kDump version {}", VERSION)?;

    let filename = config.file_path.to_string();
    let raw_contents = fs::read(filename)?;
    let mut raw_contents_iter = raw_contents.iter().peekable();

    let file_type = determine_file_type(&raw_contents)?;

    match file_type {
        // If this is a compiled kerbal machine code file
        FileType::KSM => {
            let ksm = KSMFile::from_bytes(&mut raw_contents_iter, false)?;

            let ksm_debug = KSMFileDebug::new(ksm);

            ksm_debug.dump(&mut stream, &config)?;

            Ok(())
        }
        // If it is a kerbal object file
        FileType::KO => {
            let kofile = KOFile::from_bytes(&mut raw_contents_iter, false)?;

            let ko_debug = KOFileDebug::new(kofile);

            ko_debug.dump(&mut stream, &config)?;

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
    pub reloc: bool,
    pub all_headers: bool,
    pub info: bool,
    pub demangle: bool,
    pub show_no_raw_instr: bool,
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
            reloc: matches.is_present("reloc"),
            all_headers: matches.is_present("all_headers"),
            info: matches.is_present("info"),
            demangle: matches.is_present("demangle"),
            show_no_raw_instr: matches.is_present("show_no_raw_instr"),
            show_no_labels: matches.is_present("show_no_labels"),
        }
    }
}
