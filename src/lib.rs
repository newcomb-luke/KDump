use std::io::prelude::*;
use std::{fs, error::Error, collections::HashMap, iter::Peekable, slice::Iter};
use clap::{ArgMatches};
use flate2::read::GzDecoder;

mod argument;
mod opcode;

use argument::{Argument, read_argument, argument_type_string};
use opcode::{read_all_sections, SectionType, get_instr_mnemonic, Instr, CodeSection, DebugSection};

pub static VERSION:&'static str = "1.0.0";

enum FileType {
    KSM,
    KO,
    UNKNOWN
}

pub fn run(matches: ArgMatches) -> Result<(), Box<dyn Error>> {
    println!("kDump version {}", VERSION);

    let filename = matches.value_of("INPUT").unwrap();
    let raw_contents = fs::read(filename)?;

    let file_type = determine_file_type(&raw_contents)?;

    match file_type {
        FileType::KSM => {
            return dump_ksm(matches, raw_contents);
        },
        FileType::KO => {
            return Err("KerbalObject file dumping has not yet been implemented.".into());
        },
        FileType::UNKNOWN => {
            return Err("File type not recognized.".into());
        }
    }
}

fn dump_arg_section(args_list: &Vec<Argument>, map_arg_to_index: &HashMap<i32, i32>)
{
    println!("\nArgument section:");
    println!("  {:<12}{:<24}{}", "Type", "Value", "Index");

    for i in 0..args_list.len() {
        println!("  {:<12}{:<24}{:>}", argument_type_string(&args_list[i]), args_list[i].to_string(), map_arg_to_index.get(&(i as i32)).unwrap());
    }
}

fn dump_ksm_info(args_list: &Vec<Argument>)
{
    println!("\nKSM File Info:");

    let msg: String = match args_list.get(0) {
        Some(arg) => {
            match arg {
                Argument::String(_) => String::from("  Compiled using internal kOS compiler."),
                Argument::Int32(i) => {

                    let mut e = *i;

                    let compiler = e / 0x1000;
                    e %= 0x1000;
                    let major_version = e / 0x0100;
                    e %= 0x0100;
                    let minor_version = e / 0x0010;
                    e %= 0x0010;
                    let patch_version = e;
            
                    let compiler_name = match compiler {
                        0 => "Unknown 3rd party compiler.",
                        1 => "Unknown 3rd party compiler.",
                        2 => "KASM",
                        3 => "KerbalC",
                        4 => "Gravitas",
                        5 => "Unofficial external KerbalScript compiler",
                        _ => "Compiler unrecognized. Please update this tool. If there is no updated, contact the developer.",
                    };
            
                    format!("  Compiled using: {} version {}.{}.{}", compiler_name, major_version, minor_version, patch_version)
                    
                }
                _ => String::from("  Unknown compiler. Consider updating this tool version or contacting the developer.")
            }
        },
        None => {
            String::from("  Unknown compiler. Not enough data.")
        }
    };

    println!("{}", msg);
}

fn dump_ksm_disassemble(ksm_code_sections_result: &Vec<CodeSection>, index_bytes: i32, args_list: &Vec<Argument>, map_index_to_arg: &HashMap<i32, i32>, show_addresses: bool, show_raw_insn: bool) {
    let mut current_instr_address = 1;

    for section in ksm_code_sections_result.iter() {
        println!("\n{}:", match section.section_type {
            SectionType::FUNCTION => "FUNCTION",
            SectionType::INITIALIZATION => "INITIALIZATION",
            SectionType::MAIN => "MAIN"
        });

        for instr in &section.instructions {

            let opcode: u8;
            let num_operands;
            let mut ops: Vec<i32> = Vec::new();

            match instr {
                Instr::NoOperand(op) => {
                    opcode = *op;
                    num_operands = 0;
                },
                Instr::SingleOperand(op, o1) => {
                    opcode = *op;
                    ops.push(*o1);
                    num_operands = 1;
                },
                Instr::DoubleOperand(op, o1, o2) => {
                    opcode = *op;
                    ops.push(*o1);
                    ops.push(*o2);
                    num_operands = 2;
                }
            };

            print!("  ");

            if show_addresses {
                print!("{:06x}  ", current_instr_address);
            }

            if show_raw_insn {

                let mut raw_instr_str = String::new();

                raw_instr_str.push_str(&format!("{:04x} ", opcode));

                for operand in &ops {
                    raw_instr_str.push_str(&match index_bytes {
                        1 => format!("{:04x} ", *operand as u16),
                        2 => format!("{:04x} ", *operand as u16),
                        3 => format!("{:02x}{:04x} ", (*operand / 0x100) as u8, (*operand % 0x100) as u16),
                        4 => format!("{:06x} ", *operand),
                        _ => String::from("ERROR! You have way too many arguments")
                    });
                }

                print!("{:<20} ", raw_instr_str);
            }

            print!("{:<5}", get_instr_mnemonic(opcode));

            let mut current_operand = 0;
            for operand in &ops {
                print!(" {}", args_list[*map_index_to_arg.get(operand).unwrap() as usize]);

                current_operand += 1;

                if current_operand < num_operands {
                    print!(",");
                }
            }

            println!("");

            current_instr_address += 1;
        }
        println!("");
    }
}

fn dump_ksm(matches: ArgMatches, raw_contents: Vec<u8>) -> Result<(), Box<dyn Error>> {

    let mut decoder = GzDecoder::new(&raw_contents[..]);
    let mut decompressed: Vec<u8> = Vec::new();

    decoder.read_to_end(&mut decompressed)?;

    if matches.is_present("info") ||  matches.is_present("disassemble") || matches.is_present("full_contents")
    || matches.is_present("argument_section") {
        let mut args_list: Vec<Argument> = Vec::new();
        let mut map_index_to_arg: HashMap<i32, i32> = HashMap::new();
        let mut map_arg_to_index: HashMap<i32, i32> = HashMap::new();
        let mut contents_iter = decompressed.iter().peekable();

        let index_bytes = read_arguments(&mut contents_iter, &mut args_list, &mut map_index_to_arg, &mut map_arg_to_index)?;

        if matches.is_present("argument_section") {
            dump_arg_section(&args_list, &map_arg_to_index);
        }
        
        if matches.is_present("info") {
            dump_ksm_info(&args_list);
        }

        if matches.is_present("disassemble") {
            let (ksm_code_sections_result, ksm_debug_section) = read_all_sections(&mut contents_iter, index_bytes)?;

            dump_ksm_disassemble(&ksm_code_sections_result, index_bytes, &args_list, &map_index_to_arg, !matches.is_present("show_no_addresses"), !matches.is_present("show_no_raw_insn"));
        }

    }
    else
    {
        println!("No action specified.");
    }

    Ok(())
}

fn read_arguments(contents_iter: &mut Peekable<Iter<u8>>, args_list: &mut Vec<Argument>, map_index_to_arg: &mut HashMap<i32, i32>, map_arg_to_index: &mut HashMap<i32, i32>) -> Result<i32, Box<dyn Error>> {

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
    }
    else if is_ko(contents) {
        file_type = FileType::KO;
    }

    Ok(file_type)
}

/**
 * Checks if the file is in proper GZIP format
 */
fn is_gzip(contents: &[u8]) -> bool {
    contents[0] == 0x1f && contents[1] == 0x8b && contents[2] == 0x08 && contents[3] == 0x00
}

/**
 * Checks the first 4 bytes of the file to tell if the contents are a KSM file or someone's compressed homework
 */
fn is_ksm(contents: &[u8]) -> bool {
    contents[0] == 0x6b && contents[1] == 0x03 && contents[2] == 0x58 && contents[3] == 0x45
}

/**
 * Checks the first 4 bytes of the file to tell if the contents are a KO file
 */
fn is_ko(contents: &[u8]) -> bool {
    contents[0] == 0x6b && contents[1] == 0x01 && contents[2] == 0x6f && contents[3] == 0x66
}