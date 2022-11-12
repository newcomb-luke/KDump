use clap::Parser;
use kerbalobjects::ko::KOFile;
use kerbalobjects::ksm::KSMFile;
use kerbalobjects::BufferIterator;
use std::io::Write;
use std::path::PathBuf;
use std::{error::Error, fs};
use termcolor::{Color, ColorSpec, StandardStream};

mod fio;
use fio::{determine_file_type, FileType};

mod output;
use output::KOFileDebug;
use output::KSMFileDebug;

pub static NO_COLOR: Color = Color::Rgb(255, 255, 255);

pub static VERSION: &str = env!("CARGO_PKG_VERSION");

pub static ORANGE_COLOR: Color = Color::Rgb(201, 155, 87);
pub static PURPLE_COLOR: Color = Color::Rgb(133, 80, 179);
pub static DARK_RED_COLOR: Color = Color::Rgb(201, 87, 87);
pub static LIGHT_RED_COLOR: Color = Color::Rgb(255, 147, 147);
pub static GREEN_COLOR: Color = Color::Rgb(129, 181, 154);

pub fn run(config: &CLIConfig) -> Result<(), Box<dyn Error>> {
    // We don't want color output if this is outputting to a file
    let color_choice = if atty::is(atty::Stream::Stdout) {
        termcolor::ColorChoice::Auto
    } else {
        termcolor::ColorChoice::Never
    };

    let mut stream = StandardStream::stdout(color_choice);

    let mut no_color = ColorSpec::new();
    no_color.set_fg(Some(NO_COLOR));

    writeln!(stream, "kDump version {}", VERSION)?;

    let raw_contents = fs::read(&config.file_path)?;
    let mut raw_contents_iter = BufferIterator::new(&raw_contents);

    let file_type = determine_file_type(&raw_contents)?;

    match file_type {
        FileType::KerbalMachineCode => {
            let ksm = KSMFile::parse(&mut raw_contents_iter)?;
            let ksm_debug = KSMFileDebug::new(ksm);

            ksm_debug.dump(&mut stream, config)?;

            Ok(())
        }
        FileType::KerbalObject => {
            let kofile = KOFile::parse(&mut raw_contents_iter)?;
            let ko_debug = KOFileDebug::new(kofile);

            ko_debug.dump(&mut stream, config)?;

            Ok(())
        }
        // If we have no idea what the heck the file is
        FileType::Unknown => Err("File type not recognized.".into()),
    }
}

/// This structure controls all the settings that make this program perform differently
/// These represent command line arguments read in by clap
#[derive(Debug, Parser)]
#[command(name = "kDump Utility", author, version, about, long_about = None)]
pub struct CLIConfig {
    /// The input file path, which is required
    #[arg(value_name = "FILE", help = "Sets the input file to use")]
    pub file_path: PathBuf,
    /// Whether we should disassemble the file's code sections
    /// Conflicts with disassemble_symbol and full-contents
    #[arg(
        short = 'D',
        long,
        help = "Disassembles the contents of the entire object file",
        conflicts_with("disassemble_symbol"),
        conflicts_with("full_contents")
    )]
    pub disassemble: bool,
    /// Whether we should disassemble the file's code sections, starting at a particular symbol
    /// Conflicts with disassemble and full-contents
    #[arg(
        short = 'd',
        long = "disassemble-symbol",
        help = "Disassembles at the symbol provided until the end of the section",
        require_equals = true,
        value_name = "SYMBOL",
        conflicts_with("disassemble"),
        conflicts_with("full_contents")
    )]
    pub disassemble_symbol: Option<String>,
    /// Whether we should dump the file headers
    /// KO only
    #[arg(
        short = 'f',
        long = "file-headers",
        help = "Displays summary information of the KO file header"
    )]
    pub file_headers: bool,
    /// Whether we should dump the argument section contents
    /// KSM only
    #[arg(
        short = 'a',
        long = "argument-section",
        help = "Displays the contents of the argument section of a KSM file"
    )]
    pub argument_section: bool,
    /// Whether we should display line numbers in disassembled code
    /// KSM only
    #[arg(
        short = 'l',
        long = "line-numbers",
        help = "Displays the line numbers of disassembled instructions when disassembling"
    )]
    pub line_numbers: bool,
    /// Whether we should dump the section header table
    /// KO only
    #[arg(
        long = "section-headers",
        help = "Displays the section header table of a KO file"
    )]
    pub section_headers: bool,
    /// Whether we should dump the data section of the file
    /// KO only
    #[arg(long = "data", help = "Displays each data section of a KO file")]
    pub data: bool,
    /// Whether we should display the contents of every section in the object file
    #[arg(
        short = 's',
        long = "full-contents",
        help = "Displays the contents of every section in the object file"
    )]
    pub full_contents: bool,
    /// Whether we should dump the string tables of the file
    /// KO only
    #[arg(
        short = 'S',
        long = "stabs",
        help = "Displays each string table of a KO file"
    )]
    pub stabs: bool,
    /// Whether we should dump the symbol tables of the file
    /// KO only
    #[arg(
        short = 't',
        long = "syms",
        help = "Displays each symbol table of a KO file"
    )]
    pub syms: bool,
    /// Whether we should dump the relocation data section of the file
    /// KO only
    #[arg(
        short = 'r',
        long = "reloc",
        help = "Displays each relocation data table of a KO file"
    )]
    pub reloc: bool,
    /// Whether we should display all of the section headers of the file
    /// KO only
    #[arg(
        short = 'x',
        long = "all-headers",
        help = "Displays all available KO file header information including the symbol table"
    )]
    pub all_headers: bool,
    /// Displays all available meta info of the object file including compiler comments and version information
    #[arg(
        short = 'i',
        long = "info",
        help = "Displays all available meta info of the object file including compiler comments and version information"
    )]
    pub info: bool,
    /// Whether we should attempt to demangle symbol names
    #[arg(
        short = 'C',
        long = "demangle",
        help = "Tries to demangle disassembled function and variable names"
    )]
    pub demangle: bool,
    /// A flag for if we should NOT display raw instruction bytes in the disassembly
    /// KSM only
    #[arg(
        long = "show-no-raw-instr",
        help = "When disassembling, disables showing the raw bytes that make up each instruction"
    )]
    pub show_no_raw_instr: bool,
    /// A flag for if we should NOT display instruction labels in the disassembly
    /// KSM only
    #[arg(
        long = "show-no-labels",
        help = "When disassembling, disables showing the label of each instruction"
    )]
    pub show_no_labels: bool,
}
