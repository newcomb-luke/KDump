use clap::ArgMatches;
use colored::*;
use flate2::read::GzDecoder;
use std::io::prelude::*;
use std::{collections::HashMap, error::Error, fs, iter::Peekable, slice::Iter};

mod argument;
mod opcode;

use argument::{argument_type_string, read_argument, Argument};
use opcode::{
    get_instr_mnemonic, read_all_sections, size_of_instr, CodeSection, DebugEntry, DebugSection,
    Instr, SectionType,
};

pub static VERSION: &'static str = "1.0.0";
pub static LINE_COLOR: (u8, u8, u8) = (201, 155, 87);
pub static ADDRESS_COLOR: (u8, u8, u8) = (133, 80, 179);
pub static MNEMONIC_COLOR: (u8, u8, u8) = (201, 87, 87);
pub static VARIABLE_COLOR: (u8, u8, u8) = (255, 147, 147);

enum FileType {
    KSM,
    KO,
    UNKNOWN,
}

pub fn run(matches: ArgMatches) -> Result<(), Box<dyn Error>> {
    println!("kDump version {}", VERSION);

    let filename = matches.value_of("INPUT").unwrap();
    let raw_contents = fs::read(filename)?;

    let file_type = determine_file_type(&raw_contents)?;

    match file_type {
        FileType::KSM => {
            return dump_ksm(matches, raw_contents);
        }
        FileType::KO => {
            return Err("KerbalObject file dumping has not yet been implemented.".into());
        }
        FileType::UNKNOWN => {
            return Err("File type not recognized.".into());
        }
    }
}

fn dump_arg_section(
    args_list: &Vec<Argument>,
    arg_index_bytes: i32,
    map_arg_to_index: &HashMap<i32, i32>,
) {
    println!("\nArgument section {} byte indexing:", arg_index_bytes);
    println!("  {:<12}{:<24}{}", "Type", "Value", "Index");

    for i in 0..args_list.len() {
        println!(
            "  {:<12}{:<24}{:>}",
            argument_type_string(&args_list[i]),
            args_list[i].to_string(),
            map_arg_to_index.get(&(i as i32)).unwrap()
        );
    }
}

fn dump_ksm_info(args_list: &Vec<Argument>) {
    println!("\nKSM File Info:");

    let msg: String = match args_list.get(0) {
        Some(arg) => match arg {
            Argument::String(s) => {
                if s.starts_with("@") {
                    String::from("  Compiled using internal kOS compiler.")
                } else {
                    format!("  {}", s)
                }
            }
            _ => String::from("  Unknown compiler."),
        },
        None => String::from("  Unknown compiler. Not enough data."),
    };

    println!("{}", msg);
}

fn print_line_number(
    debug_entry: &DebugEntry,
    address: i32,
    ops_len: i32,
    max_line_number_width: usize,
) -> bool {
    let mut state = 0;

    for (range_start, range_end) in &debug_entry.ranges {
        let range_start = *range_start;
        let range_end = *range_end;

        let middle_range = ((range_end - range_start) / 2) + range_start;

        state = match address {
            addr if addr == range_end && range_start == range_end => 3,
            addr if addr == range_start => 0,
            addr if middle_range >= addr && (middle_range <= (addr + ops_len)) => 2,
            addr if addr + ops_len < range_end && addr > range_start => 1,
            addr if addr + ops_len == range_end => 4,
            _ => 0,
        };
    }

    let mut before_text = String::new();

    if state == 2 {
        before_text = format!("  Line {} ", debug_entry.line_number)
    }

    let after_text = String::from(match state {
        0 => " ╔═ ",
        1 => " ║  ",
        2 => "═╣  ",
        3 => "════",
        4 => " ╚═ ",
        _ => "     ",
    });

    let (_r, _g, _b) = LINE_COLOR;

    print!(
        "{}",
        format!(
            "{:<1$}{2}",
            before_text,
            max_line_number_width + 8,
            after_text
        )
        .truecolor(_r, _g, _b)
    );

    state == 3 || state == 4
}

fn print_colored_argument(argument: &Argument) {
    match argument {
        Argument::String(s) | Argument::StringValue(s) => {
            if s.starts_with("$") {
                let (_r, _g, _b) = VARIABLE_COLOR;
                print!("{}", format!("{}", s).truecolor(_r, _g, _b));
            } else {
                print!("{}", argument);
            }
        }
        _ => {
            print!("{}", argument);
        }
    }
}

fn check_section_contains(
    section: &CodeSection,
    symbol: &String,
    args_list: &Vec<Argument>,
    map_index_to_arg: &HashMap<i32, i32>,
) -> bool {
    if symbol == "main" {
        if section.section_type == SectionType::MAIN {
            return true;
        }
    }

    for instr in &section.instructions {
        let mut ops: Vec<i32> = Vec::new();

        match instr {
            Instr::SingleOperand(_, o1) => {
                ops.push(*o1);
            }
            Instr::DoubleOperand(_, o1, o2) => {
                ops.push(*o1);
                ops.push(*o2);
            }
            _ => {}
        };

        for operand in ops.iter() {
            let argument = &args_list[*map_index_to_arg.get(operand).unwrap() as usize];

            match argument {
                Argument::String(s) | Argument::StringValue(s) => {
                    if s.to_string() == symbol.to_string() {
                        return true;
                    }
                }
                _ => {}
            }
        }
    }

    false
}

fn dump_ksm_disassemble(
    ksm_code_sections_result: &Vec<CodeSection>,
    ksm_debug_section: &DebugSection,
    index_bytes: i32,
    args_list: &Vec<Argument>,
    map_index_to_arg: &HashMap<i32, i32>,
    show_addresses: bool,
    show_raw_insn: bool,
    show_line_numbers: bool,
    disassembly_symbol_option: Option<String>,
) {
    let null_debug_entry = DebugEntry {
        line_number: -1,
        number_ranges: -1,
        ranges: Vec::new(),
    };

    let mut current_instr_address = 0;
    let mut current_debug_entry: &DebugEntry = &null_debug_entry;
    let mut current_debug_entry_index = 0;
    let mut max_line_number_width = 0;

    let disassembling_by_symbol = match &disassembly_symbol_option {
        Some(_) => true,
        None => false,
    };

    let disassembly_symbol = if disassembling_by_symbol {
        disassembly_symbol_option.unwrap()
    } else {
        String::new()
    };

    if show_line_numbers {
        for entry in &ksm_debug_section.debug_entries {
            if format!("{}", entry.line_number).len() > max_line_number_width {
                max_line_number_width = format!("{}", entry.line_number).len();
            }
        }

        current_debug_entry = match ksm_debug_section.debug_entries.get(0) {
            Some(debug_entry) => debug_entry,
            None => &null_debug_entry,
        };
    }

    for section in ksm_code_sections_result.iter() {
        current_instr_address += match section.section_type {
            SectionType::FUNCTION => 2,
            SectionType::INITIALIZATION => 4,
            SectionType::MAIN => 6,
        };

        let is_disassembling = !disassembling_by_symbol
            || check_section_contains(section, &disassembly_symbol, args_list, map_index_to_arg);

        if is_disassembling {
            println!(
                "\n{}:",
                match section.section_type {
                    SectionType::FUNCTION => "FUNCTION",
                    SectionType::INITIALIZATION => "INITIALIZATION",
                    SectionType::MAIN => "MAIN",
                }
            );

            for instr in &section.instructions {
                let opcode: u8;
                let num_operands;
                let mut ops: Vec<i32> = Vec::new();

                match instr {
                    Instr::NoOperand(op) => {
                        opcode = *op;
                        num_operands = 0;
                    }
                    Instr::SingleOperand(op, o1) => {
                        opcode = *op;
                        ops.push(*o1);
                        num_operands = 1;
                    }
                    Instr::DoubleOperand(op, o1, o2) => {
                        opcode = *op;
                        ops.push(*o1);
                        ops.push(*o2);
                        num_operands = 2;
                    }
                };

                if show_line_numbers {
                    if print_line_number(
                        &current_debug_entry,
                        current_instr_address,
                        size_of_instr(instr, index_bytes) - 1,
                        max_line_number_width,
                    ) {
                        current_debug_entry_index += 1;
                        current_debug_entry = match &ksm_debug_section
                            .debug_entries
                            .get(current_debug_entry_index)
                        {
                            Some(entry) => entry,
                            None => {
                                current_debug_entry_index -= 1;
                                current_debug_entry
                            }
                        };
                    }
                } else {
                    print!("  ");
                }

                if show_addresses {
                    let (_r, _g, _b) = ADDRESS_COLOR;
                    print!(
                        "{}",
                        format!("{:06x}  ", current_instr_address).truecolor(_r, _g, _b)
                    );
                }

                if show_raw_insn {
                    let mut raw_instr_str = String::new();

                    raw_instr_str.push_str(&format!("{:02x} ", opcode));

                    for operand in &ops {
                        raw_instr_str.push_str(&match index_bytes {
                            1 => format!("{:02x} ", *operand as u8),
                            2 => format!("{:04x} ", *operand as u16),
                            3 => format!(
                                "{:02x}{:04x} ",
                                (*operand / 0x100) as u8,
                                (*operand % 0x100) as u16
                            ),
                            4 => format!("{:08x} ", *operand),
                            _ => String::from("ERROR! You have way too many arguments"),
                        });
                    }

                    print!("{:<1$} ", raw_instr_str, (index_bytes * 6 + 3) as usize);
                }

                {
                    let (_r, _g, _b) = MNEMONIC_COLOR;
                    print!(
                        "{}",
                        format!("{:<5}", get_instr_mnemonic(opcode)).truecolor(_r, _g, _b)
                    );
                }

                let mut current_operand = 0;
                for operand in &ops {
                    let argument_ref = &args_list[*map_index_to_arg.get(operand).unwrap() as usize];

                    print_colored_argument(argument_ref);

                    current_operand += 1;

                    if current_operand < num_operands {
                        print!(",");
                    }
                }

                println!("");

                current_instr_address += size_of_instr(instr, index_bytes);
            }
        } else {
            current_instr_address += section.size;
        }
    }
}

fn dump_ksm_debug(ksm_debug_section: &DebugSection) {
    println!("\nDebug section:");

    for entry in &ksm_debug_section.debug_entries {
        print!(
            "  Line {}, {} range{}:",
            entry.line_number,
            entry.number_ranges,
            if entry.number_ranges > 1 { "s" } else { "" }
        );

        let mut current_range = 0;

        for (range_start, range_end) in &entry.ranges {
            print!(" [{:06x}, {:06x}]", *range_start, *range_end);

            current_range += 1;

            if current_range < entry.number_ranges {
                print!(",");
            }
        }

        println!("");
    }
}

fn dump_ksm(matches: ArgMatches, raw_contents: Vec<u8>) -> Result<(), Box<dyn Error>> {
    let mut decoder = GzDecoder::new(&raw_contents[..]);
    let mut decompressed: Vec<u8> = Vec::new();

    decoder.read_to_end(&mut decompressed)?;

    if matches.is_present("info")
        || matches.is_present("disassemble")
        || matches.is_present("full_contents")
        || matches.is_present("argument_section")
        || matches.is_present("disassemble_symbol")
    {
        let mut args_list: Vec<Argument> = Vec::new();
        let mut map_index_to_arg: HashMap<i32, i32> = HashMap::new();
        let mut map_arg_to_index: HashMap<i32, i32> = HashMap::new();
        let mut contents_iter = decompressed.iter().peekable();

        let index_bytes = read_arguments(
            &mut contents_iter,
            &mut args_list,
            &mut map_index_to_arg,
            &mut map_arg_to_index,
        )?;

        if matches.is_present("argument_section") || matches.is_present("full_contents") {
            dump_arg_section(&args_list, index_bytes, &map_arg_to_index);
        }

        if matches.is_present("info") {
            dump_ksm_info(&args_list);
        }

        if matches.is_present("disassemble")
            || matches.is_present("full_contents")
            || matches.is_present("disassemble_symbol")
        {
            let (ksm_code_sections_result, ksm_debug_section) =
                read_all_sections(&mut contents_iter, index_bytes)?;

            let disassemble_symbol: Option<String>;

            disassemble_symbol = match matches.value_of("disassemble_symbol") {
                Some(s) => Some(String::from(s)),
                None => None,
            };

            dump_ksm_disassemble(
                &ksm_code_sections_result,
                &ksm_debug_section,
                index_bytes,
                &args_list,
                &map_index_to_arg,
                !matches.is_present("show_no_addresses"),
                !matches.is_present("show_no_raw_insn"),
                matches.is_present("line_numbers"),
                disassemble_symbol,
            );

            if matches.is_present("full_contents") {
                dump_ksm_debug(&ksm_debug_section);
            }
        }

        println!("");
    } else {
        println!("\nNo action specified.");
    }

    Ok(())
}

fn read_arguments(
    contents_iter: &mut Peekable<Iter<u8>>,
    args_list: &mut Vec<Argument>,
    map_index_to_arg: &mut HashMap<i32, i32>,
    map_arg_to_index: &mut HashMap<i32, i32>,
) -> Result<i32, Box<dyn Error>> {
    for _ in 0..6 {
        contents_iter.next();
    }

    let index_bytes = *contents_iter.next().unwrap() as i32;

    let mut current_index: i32 = 3;
    let mut current_argument_number = 0;

    while **contents_iter.peek().unwrap() != b'%' {
        let (arg, len) = read_argument(contents_iter)?;

        args_list.push(arg);

        map_index_to_arg.insert(current_index, current_argument_number);
        map_arg_to_index.insert(current_argument_number, current_index);

        current_index += len;
        current_argument_number += 1;
    }

    Ok(index_bytes)
}

fn determine_file_type(contents: &Vec<u8>) -> Result<FileType, Box<dyn Error>> {
    let mut file_type = FileType::UNKNOWN;

    if is_gzip(contents) {
        let mut decoder = GzDecoder::new(&contents[..]);
        let mut decompressed = [0, 0, 0, 0];

        decoder.read_exact(&mut decompressed)?;

        if is_ksm(&decompressed) {
            file_type = FileType::KSM;
        }
    } else if is_ko(contents) {
        file_type = FileType::KO;
    }

    Ok(file_type)
}

/// Checks if the file is in proper GZIP format
fn is_gzip(contents: &[u8]) -> bool {
    contents[0] == 0x1f && contents[1] == 0x8b && contents[2] == 0x08 && contents[3] == 0x00
}

/// Checks the first 4 bytes of the file to tell if the contents are a KSM file or someone's compressed homework
fn is_ksm(contents: &[u8]) -> bool {
    contents[0] == 0x6b && contents[1] == 0x03 && contents[2] == 0x58 && contents[3] == 0x45
}

/// Checks the first 4 bytes of the file to tell if the contents are a KO file
fn is_ko(contents: &[u8]) -> bool {
    contents[0] == 0x6b && contents[1] == 0x01 && contents[2] == 0x6f && contents[3] == 0x66
}
