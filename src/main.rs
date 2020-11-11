use clap::{App, Arg};
use std::process;

use kdump::{run, CLIConfig};

fn main() {
    let matches = App::new("kDump Utility")
                            .version(kdump::VERSION)
                            .author("Luke Newcomb")
                            .about("Dumps the contents of an object file as specified.")
                            .arg(Arg::with_name("INPUT")
                                .help("Sets the input file to use")
                                .required(true)
                                .index(1))
                            .arg(Arg::with_name("disassemble")
                                .help("Disassembles the contents of the entire object file")
                                .short("D")
                                .long("disassemble"))
                            .arg(Arg::with_name("disassemble_symbol")
                                .help("Disassembles starting at the symbol provided until the end of the section.")
                                .short("d")
                                .long("disassemble-symbol")
                                .require_equals(true)
                                .takes_value(true)
                                .value_name("SYMBOL")
                                .conflicts_with("disassemble"))
                            .arg(Arg::with_name("file_headers")
                                .help("Displays summary information of the overall header of the KO file.")
                                .short("f")
                                .long("file-headers"))
                            .arg(Arg::with_name("argument_section")
                                .help("Displays the contents of the argument section of a KSM file.")
                                .short("a")
                                .long("argument-section"))
                            .arg(Arg::with_name("line_numbers")
                                .help("Displays the line numbers of disassembled instructions when disassembling.")
                                .short("l")
                                .long("line-numbers"))
                            .arg(Arg::with_name("section_headers")
                                .help("Displays the section headers of each section in the KO file.")
                                .short("h")
                                .long("section-headers"))
                            .arg(Arg::with_name("full_contents")
                                .help("Displays the contents of every section in the object file.")
                                .short("s")
                                .long("full-contents"))
                            .arg(Arg::with_name("stabs")
                                .help("Displays the contents of all of the string tables in the KO file.")
                                .short("S")
                                .long("stabs"))
                            .arg(Arg::with_name("syms")
                                .help("Displays the contents of all symbol tables in the KO file.")
                                .short("t")
                                .long("syms"))
                            .arg(Arg::with_name("all_headers")
                                .help("Displays all available KO file header information including the symbol table.")
                                .short("x")
                                .long("all-headers"))
                            .arg(Arg::with_name("info")
                                .help("Displays all available meta info of the object file including compiler comments and version information.")
                                .short("i")
                                .long("info"))
                            .arg(Arg::with_name("demangle")
                                .help("Tries to demangle disassembled function and variable names.")
                                .short("C")
                                .long("demangle"))
                            .arg(Arg::with_name("show_no_raw_insn")
                                .help("When disassembling, enables showing the raw bytes that make up each instruction")
                                .long("show-no-raw-insn"))
                            .arg(Arg::with_name("show_no_labels")
                                .help("When disassembling, enables showing the label of each instruction in the object file.")
                                .long("show-no-labels"))
                            .get_matches();

    let config = CLIConfig::new(matches);

    if let Err(e) = run(&config) {
        eprintln!("Application error: {}", e);

        process::exit(1);
    }
}
