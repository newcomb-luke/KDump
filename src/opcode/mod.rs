use std::{error::Error, iter::Peekable, slice::Iter};

enum Instr {
    NoOperand(u8),
    SingleOperand(u8, i32),
    DoubleOperand(u8, i32, i32)
}

enum SectionType {
    FUNCTION,
    INITIALIZATION,
    MAIN,
    DEBUG
}

struct CodeSection {
    section_type: SectionType,
    instructions: Vec<Instruction>
}

struct DebugSection {
    section_type: SectionType,
    range_size: i32,
    debug_entries: Vec<DebugEntry>
}

struct DebugEntry {
    line_number: i32,
    number_ranges: i32,
    ranges: Vec<(i32,i32)>
}

fn read_instr(byte_iter: &mut Peekable<Iter<u8>>) -> Option<Instr> {
    match byte_iter.next() {
        Some(v) => Some(Argument::Byte(*v as i8)),
        None => None
    }
}

fn read_all_sections(byte_iter: &mut Peekable<Iter<u8>>) -> Vec<Section> {
    
    loop {
        // Consume '%'
        byte_iter.next().unwrap();

        let next_char = byte_iter.next().unwrap();
    }
    

}

fn read_code_section(byte_iter: &mut Peekable<Iter<u8>>) -> Option<Instr> {

}