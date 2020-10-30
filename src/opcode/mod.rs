use std::{convert::TryInto, error::Error, iter::Peekable, slice::Iter};

pub enum Instr {
    NoOperand(u8),
    SingleOperand(u8, i32),
    DoubleOperand(u8, i32, i32),
}

#[derive(PartialEq)]
pub enum SectionType {
    FUNCTION,
    INITIALIZATION,
    MAIN,
}

pub struct CodeSection {
    pub section_type: SectionType,
    pub instructions: Vec<Instr>,
    pub size: i32,
}

pub struct DebugSection {
    pub range_size: i32,
    pub debug_entries: Vec<DebugEntry>,
}

pub struct DebugEntry {
    pub line_number: i32,
    pub number_ranges: i32,
    pub ranges: Vec<(i32, i32)>,
}

pub fn size_of_instr(instr: &Instr, index_bytes: i32) -> i32 {
    let num_operands = match instr {
        Instr::NoOperand(_) => 0,
        Instr::SingleOperand(_, _) => 1,
        Instr::DoubleOperand(_, _, _) => 2,
    };

    num_operands * index_bytes + 1
}

fn read_instr(byte_iter: &mut Peekable<Iter<u8>>, index_bytes: i32) -> Option<Instr> {
    let opcode = match byte_iter.next() {
        Some(b) => *b,
        None => {
            return None;
        }
    };

    let num_operands = get_num_operands(opcode);

    match num_operands {
        0 => Some(Instr::NoOperand(opcode)),
        1 => {
            let arg_index = match read_bytes_as_i32(byte_iter, index_bytes as usize) {
                Some(v) => v,
                None => return None,
            };

            Some(Instr::SingleOperand(opcode, arg_index))
        }
        2 => {
            let arg1_index = match read_bytes_as_i32(byte_iter, index_bytes as usize) {
                Some(v) => v,
                None => return None,
            };

            let arg2_index = match read_bytes_as_i32(byte_iter, index_bytes as usize) {
                Some(v) => v,
                None => return None,
            };

            Some(Instr::DoubleOperand(opcode, arg1_index, arg2_index))
        }
        _ => None,
    }
}

pub fn read_all_sections(
    byte_iter: &mut Peekable<Iter<u8>>,
    arg_index_bytes: i32,
) -> Result<(Vec<CodeSection>, DebugSection), Box<dyn Error>> {
    let mut sections: Vec<CodeSection> = Vec::new();
    let mut section_number = 1;
    let debug_section: DebugSection;

    loop {
        // Consume '%'
        match byte_iter.next() {
            Some(_) => {}
            None => {
                return Err(format!("Reached EOF before section {} ended.", section_number).into());
            }
        }

        let next_char = match byte_iter.next() {
            Some(v) => v,
            None => {
                return Err(format!("Reached EOF before section {} ended.", section_number).into());
            }
        };

        let peeked_char = match byte_iter.peek() {
            Some(v) => v,
            None => {
                return Err(format!("Reached EOF before section {} ended.", section_number).into());
            }
        };

        if **peeked_char != b'%' {
            if *next_char == b'D' {
                match read_debug_section(byte_iter) {
                    Some(v) => {
                        debug_section = v;
                        break;
                    }
                    None => return Err("Reached EOF before debug section ended.".into()),
                }
            } else {
                let section_type = match *next_char {
                    b'F' => SectionType::FUNCTION,
                    b'I' => SectionType::INITIALIZATION,
                    b'M' => SectionType::MAIN,
                    wrong => {
                        return Err(format!(
                            "Invalid section identifier encountered: {} ({})",
                            wrong as char, wrong
                        )
                        .into());
                    }
                };

                match read_code_section(byte_iter, arg_index_bytes, section_type) {
                    (v, msg) => match v {
                        Some(code_section) => {
                            sections.push(code_section);
                        }
                        None => {
                            return Err(msg.into());
                        }
                    },
                }
            }

            section_number += 1;
        }
    }

    Ok((sections, debug_section))
}

fn read_bytes_as_i32(byte_iter: &mut Peekable<Iter<u8>>, data_size: usize) -> Option<i32> {
    let mut bytes: Vec<u8> = vec![0; data_size];

    for j in 0..data_size {
        bytes[j] = match byte_iter.next() {
            Some(v) => *v,
            None => return None,
        }
    }

    match data_size {
        1 => Some(bytes[0] as i32),
        2 => Some(i16::from_le_bytes(
            bytes[0..2]
                .try_into()
                .expect("If this fails we have a lot of problems."),
        ) as i32),
        3 => {
            let mut bytes_ext: [u8; 4] = [0; 4];

            for (idx, b) in bytes.iter().enumerate() {
                bytes_ext[idx] = *b;
            }

            Some(i32::from_le_bytes(bytes_ext))
        }
        4 => Some(i32::from_le_bytes(
            bytes[0..4]
                .try_into()
                .expect("If this fails we have a TON of problems."),
        )),
        _ => None,
    }
}

fn get_num_operands(opcode: u8) -> i32 {
    match opcode {
        0x31 => 0,
        0x32 => 0,
        0x33 => 0,
        0x34 => 1,
        0x35 => 0,
        0x36 => 1,
        0x37 => 1,
        0x38 => 0,
        0x39 => 0,
        0x3a => 1,
        0x3b => 1,
        0x3c => 0,
        0x3d => 0,
        0x3e => 0,
        0x3f => 0,
        0x40 => 0,
        0x41 => 0,
        0x42 => 0,
        0x43 => 0,
        0x44 => 0,
        0x45 => 0,
        0x46 => 0,
        0x47 => 0,
        0x48 => 0,
        0x49 => 0,
        0x4a => 0,
        0x4b => 0,
        0x4c => 2,
        0x4d => 1,
        0x4e => 1,
        0x4f => 0,
        0x50 => 0,
        0x51 => 0,
        0x52 => 0,
        0x53 => 2,
        0x54 => 0,
        0x55 => 0,
        0x56 => 0,
        0x57 => 1,
        0x58 => 1,
        0x59 => 1,
        0x5a => 2,
        0x5b => 1,
        0x5c => 1,
        0x5d => 2,
        0x5e => 1,
        0x5f => 0,
        0x60 => 0,
        0x61 => 0,
        0x62 => 0,

        0xce => 1,
        0xcd => 2,
        0xf0 => 1,
        _ => -1,
    }
}

pub fn get_instr_mnemonic(opcode: u8) -> String {
    String::from(match opcode {
        0x31 => "eof",
        0x32 => "eop",
        0x33 => "nop",
        0x34 => "sto",
        0x35 => "uns",
        0x36 => "gmb",
        0x37 => "smb",
        0x38 => "gidx",
        0x39 => "sidx",
        0x3a => "bfa",
        0x3b => "jmp",
        0x3c => "add",
        0x3d => "sub",
        0x3e => "mul",
        0x3f => "div",
        0x40 => "pow",
        0x41 => "cgt",
        0x42 => "clt",
        0x43 => "cge",
        0x44 => "cle",
        0x45 => "ceq",
        0x46 => "cne",
        0x47 => "neg",
        0x48 => "bool",
        0x49 => "not",
        0x4a => "and",
        0x4b => "or",
        0x4c => "call",
        0x4d => "ret",
        0x4e => "push",
        0x4f => "pop",
        0x50 => "dup",
        0x51 => "swap",
        0x52 => "eval",
        0x53 => "addt",
        0x54 => "rmvt",
        0x55 => "wait",
        0x56 => "endw",
        0x57 => "gmet",
        0x58 => "stol",
        0x59 => "stog",
        0x5a => "bscp",
        0x5b => "escp",
        0x5c => "stoe",
        0x5d => "phdl",
        0x5e => "btr",
        0x5f => "exst",
        0x60 => "argb",
        0x61 => "targ",
        0x62 => "tcan",

        0xce => "prl",
        0xcd => "pdrl",
        0xf0 => "lbrt",
        _ => "bogus",
    })
}

fn read_code_section(
    byte_iter: &mut Peekable<Iter<u8>>,
    index_bytes: i32,
    section_type: SectionType,
) -> (Option<CodeSection>, String) {
    let mut section_size = 0;
    let mut instructions_list: Vec<Instr> = Vec::new();

    let mut peeked_char = match byte_iter.peek() {
        Some(v) => v,
        None => {
            return (None, String::from("Reached EOF before section ended."));
        }
    };

    while **peeked_char != b'%' {
        instructions_list.push(match read_instr(byte_iter, index_bytes) {
            Some(v) => {
                section_size += size_of_instr(&v, index_bytes);

                v
            }
            None => return (None, String::from("Reached EOF while reading instruction.")),
        });

        peeked_char = match byte_iter.peek() {
            Some(v) => v,
            None => {
                return (None, String::from("Reached EOF before section ended."));
            }
        };
    }

    (
        Some(CodeSection {
            section_type: section_type,
            instructions: instructions_list,
            size: section_size,
        }),
        String::from(""),
    )
}

fn read_debug_section(byte_iter: &mut Peekable<Iter<u8>>) -> Option<DebugSection> {
    let range_size = match byte_iter.next() {
        Some(size) => *size as i32,
        None => return None,
    };

    let mut entry_list: Vec<DebugEntry> = Vec::new();

    while byte_iter.peek() != None {
        let mut ranges_list: Vec<(i32, i32)> = Vec::new();

        let line_number = match read_bytes_as_i32(byte_iter, 2) {
            Some(num) => num,
            None => return None,
        };

        let number_ranges = match byte_iter.next() {
            Some(num) => *num as i32,
            None => return None,
        };

        for _ in 0..number_ranges {
            let range_start = match read_bytes_as_i32(byte_iter, range_size as usize) {
                Some(v) => v,
                None => return None,
            };
            let range_end = match read_bytes_as_i32(byte_iter, range_size as usize) {
                Some(v) => v,
                None => return None,
            };

            ranges_list.push((range_start, range_end));
        }

        entry_list.push(DebugEntry {
            line_number: line_number,
            number_ranges: number_ranges,
            ranges: ranges_list,
        });
    }

    Some(DebugSection {
        range_size: range_size,
        debug_entries: entry_list,
    })
}
