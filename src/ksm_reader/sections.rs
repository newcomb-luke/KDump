use std::{collections::HashMap, error::Error};

use crate::{
    Argument, Instr, KSMFileReader, Terminal, Value, ADDRESS_COLOR, LINE_COLOR, MNEMONIC_COLOR,
    TYPE_COLOR, VARIABLE_COLOR,
};
use termcolor::ColorSpec;

#[derive(PartialEq, Eq, Debug, Copy, Clone)]
pub enum SectionType {
    FUNCTION,
    INITIALIZATION,
    MAIN,
}

pub struct CodeSection {
    section_type: SectionType,
    instructions: Vec<Instr>,
    size: u32,
    index_to_offset: HashMap<usize, u32>,
    num_real_instructions: u32,
}

impl CodeSection {
    pub fn new(section_type: SectionType, instructions: Vec<Instr>, size: u32) -> CodeSection {
        let mut index_to_offset: HashMap<usize, u32> = HashMap::with_capacity(instructions.len());

        let mut offset = 0;
        let mut num_real_instructions = 0;

        for (index, instruction) in instructions.iter().enumerate() {
            index_to_offset.insert(index, offset);

            offset += instruction.size() as u32;

            // If it is not a labelreset instruction
            if instruction.get_opcode() != 0xf0 {
                num_real_instructions += 1;
            }
        }

        CodeSection {
            section_type,
            instructions,
            size,
            index_to_offset,
            num_real_instructions
        }
    }

    pub fn read(reader: &mut KSMFileReader) -> Result<CodeSection, Box<dyn Error>> {
        let mut size: u32 = 6;

        let mut instructions: Vec<Instr> = Vec::new();

        let section_type = CodeSection::read_section_type(reader)?;

        while reader.peek()? != b'%' {
            instructions.push(Instr::read(reader)?);

            size += instructions.last().unwrap().size() as u32;
        }

        if section_type == SectionType::FUNCTION {
            // %I, %M
            reader.pop(4)?;
        } else if section_type == SectionType::INITIALIZATION {
            // %M
            reader.pop(2)?;
        }

        Ok(CodeSection::new(section_type, instructions, size))
    }

    pub fn read_section_type(reader: &mut KSMFileReader) -> Result<SectionType, Box<dyn Error>> {
        let mut tries = 0;

        if reader.next()? != b'%' || reader.next()? != b'F' {
            return Err(format!(
                "Expected start of function section at index {}",
                reader.get_current_index()
            )
            .into());
        }

        while reader.peek()? == b'%' {
            // Pop off that %[whatever]
            reader.pop(2)?;
            tries += 1;
        }

        Ok(match tries {
            0 => SectionType::FUNCTION,
            1 => SectionType::INITIALIZATION,
            2 => SectionType::MAIN,
            _ => {
                return Err("Expected code section, none found!".into());
            }
        })
    }

    pub fn get_type(&self) -> SectionType {
        self.section_type
    }

    pub fn get_instructions(&self) -> &Vec<Instr> {
        &self.instructions
    }

    pub fn size(&self) -> u32 {
        self.size
    }

    pub fn number_real_instructions(&self) -> u32 {
        self.num_real_instructions
    }

    pub fn get_offset(&self, index: usize) -> u32 {
        *self.index_to_offset.get(&index).unwrap()
    }

    pub fn contains(&self, symbol: &String, argument_section: &ArgumentSection) -> bool {
        for instruction in self.instructions.iter() {
            for operand in instruction.get_operands().iter() {
                let argument = argument_section.get_argument(*operand);

                match argument.get_value() {
                    Value::String(s) | Value::StringValue(s) => {
                        if s.contains(symbol) {
                            return true;
                        }
                    }
                    _ => {}
                }
            }
        }

        false
    }

    pub fn get_function_name(&self, argument_section: &ArgumentSection) -> String {
        let mut func_name = String::new();

        let first_instruction = self.instructions.get(0);

        match first_instruction {
            Some(instruction) => {
                // Tests if the first instruction is a lbrt (LABELRESET) instruction
                if instruction.get_opcode() == 0xf0 {
                    // Gets the label from inside the instruction's argument
                    let label = argument_section
                        .get_argument(*instruction.get_operands().get(0).unwrap())
                        .get_repr();

                    // If it is a KS generated function
                    if label.contains("`") {
                        func_name = format!(" {}", label.split('`').next().unwrap());
                    } else {
                        func_name = format!(" {}", label);
                    }
                }
            }
            None => {}
        }

        func_name
    }

    pub fn dump(
        &self,
        global_offset: u32,
        global_instruction_index: u32,
        show_labels: bool,
        show_raw: bool,
        show_line_numbers: bool,
        argument_section: &ArgumentSection,
        debug_section: &DebugSection,
    ) -> Result<(), Box<dyn Error>> {

        let mut term = Terminal::new(ColorSpec::new());

        let mut variable_color = ColorSpec::new();
        variable_color.set_fg(Some(VARIABLE_COLOR));

        let mut offset = global_offset;

        offset += match self.section_type {
            SectionType::FUNCTION => 2,
            SectionType::INITIALIZATION => 4,
            SectionType::MAIN => 6,
        };

        if self.section_type != SectionType::FUNCTION {
            term.writeln(&format!("\n{:?}:", self.section_type))?;
        } else {
            term.write(&format!(
                "\n{:?}",
                self.section_type
            ))?;

            term.write_colored(&self.get_function_name(argument_section), &variable_color)?;

            term.writeln(&String::from(":"))?;
        }

        let mut addr = offset;

        let mut instruction_index = global_instruction_index;

        let mut label = format!("@{:06}", instruction_index);

        let mut variable_color = ColorSpec::new();
        variable_color.set_fg(Some(VARIABLE_COLOR));

        let mut mnemonic_color = ColorSpec::new();
        mnemonic_color.set_fg(Some(MNEMONIC_COLOR));

        let mut line_color = ColorSpec::new();
        line_color.set_fg(Some(LINE_COLOR));

        let mut address_color = ColorSpec::new();
        address_color.set_fg(Some(ADDRESS_COLOR));

        for (index, instruction) in self.instructions.iter().enumerate() {
            if show_line_numbers {
                let max_line_number_width = debug_section.max_line_number.to_string().len();

                match debug_section.find(addr) {
                    Some((entry, range)) => {
                        let (range_start, range_end) = range;
                        let middle_range = ((range_end - range_start) / 2) + range_start;

                        let operands_length = (instruction.size() - 1) as u32;

                        let state = match addr {
                            addr if addr == range_start
                                && range_start + operands_length == range_end =>
                            {
                                3
                            }
                            addr if addr == range_start => {
                                let next_instruction = self.instructions.get(index + 1).unwrap();

                                if addr + operands_length + next_instruction.size() as u32
                                    == range_end
                                {
                                    5
                                } else {
                                    0
                                }
                            }
                            addr if addr + operands_length == range_end => 4,
                            addr if middle_range >= addr
                                && (middle_range <= (addr + operands_length)) =>
                            {
                                2
                            }
                            addr if addr + operands_length < range_end && addr > range_start => 1,
                            _ => 6,
                        };

                        // term.writeln(&format!("Addr: {:x}, Addr+Len: {:x}, State: {}, Line: {}, range: [{:06x}, {:06x}]", addr, addr + operands_length, state, entry.line_number, range_start, range_end))?;

                        let before_text = if state == 2 || state == 5 || state == 3 {
                            format!("   {} ", entry.line_number)
                        } else {
                            String::from("")
                        };

                        let after_text = String::from(match state {
                            0 => " ╔═ ",
                            1 => " ║  ",
                            2 => "═╣  ",
                            3 => "═══ ",
                            4 => " ╚═ ",
                            5 => "═╦═ ",
                            _ => "    ",
                        });

                        term.write_colored(
                            &format!(
                                "{:>1$}{2}",
                                before_text,
                                max_line_number_width + 4,
                                after_text
                            ),
                            &line_color,
                        )?;
                    }

                    None => {
                        term.write(&String::from("    "))?;
                    }
                };
            } else {
                term.write(&String::from("  "))?;
            }

            // So it turns out we only need to keep track of the actual "address" of the instruction for the debug section.
            // This is very annoying because branch instructions use the labels of the instructions instead :)
            // So this would be more useful showing that, so that is what it does now.

            if show_labels {

                // If it is a labelreset, then it doesn't really have a label, so just show nothing?
                if instruction.get_opcode() == 0xf0 {
                    term.write(&format!("{:08}", ""))?;
                }
                else {
                    term.write_colored(&format!("{} ", label), &address_color)?;
                }
            }

            // If the instruction is a labelreset, reset the label.
            if instruction.get_opcode() == 0xf0 {
                label = match argument_section.get_argument(*instruction.get_operands().get(0).unwrap()).get_value() {
                    Value::String(s) => s.to_owned(),
                    _ => unreachable!()
                };

                // If it is something like @0013
                if label.starts_with("@") {
                    // Make the formatting all pretty by making @0013, @000013
                    label.insert_str(1, "00");
                }

                // Get the first 6 characters of the label
                // Might change this later
                label.truncate(7);
            } else {
                label = format!("@{:>06}", instruction_index+1);
            }

            // if show_addr {
            //     term.write_colored(
            //         &format!(
            //             "{:06x}  ",
            //             offset + self.index_to_offset.get(&index).unwrap()
            //         ),
            //         &address_color
            //     )?;
            // }

            if show_raw {
                term.write(&instruction.raw_str())?;
            }

            term.write_colored(&format!("{:<4} ", Instr::get_mnemonic(&instruction)), &mnemonic_color)?;

            for (index, operand) in instruction.get_operands().iter().enumerate() {

                let argument = argument_section.get_argument(*operand);

                if argument.is_variable() {
                    term.write_colored(&argument.get_repr(), &variable_color)?;
                } else {
                    term.write(&argument.get_repr())?;
                }

                if index < instruction.num_operands() - 1 {
                    term.write(&String::from(", "))?;
                }
            }

            term.writeln(&String::from(""))?;

            addr += instruction.size() as u32;

            // Label resets don't count as an instruction so don't count it here
            if instruction.get_opcode() != 0xf0 {
                instruction_index += 1;
            }
            
        }

        Ok(())
    }
}

pub struct DebugSection {
    range_size: u8,
    max_line_number: u16,
    debug_entries: Vec<DebugEntry>,
}

impl DebugSection {
    pub fn new(
        range_size: u8,
        max_line_number: u16,
        debug_entries: Vec<DebugEntry>,
    ) -> DebugSection {
        DebugSection {
            range_size,
            max_line_number,
            debug_entries,
        }
    }

    pub fn read(reader: &mut KSMFileReader) -> Result<DebugSection, Box<dyn Error>> {
        if reader.next()? != b'%' || reader.next()? != b'D' {
            return Err("Debug section expected".into());
        }

        let range_size = reader.next()?;
        let mut debug_entries: Vec<DebugEntry> = Vec::new();
        let mut max_line_number = 0;

        reader.set_range_bytes(range_size);

        while !reader.eof() {
            let entry = DebugEntry::read(reader)?;

            if entry.line_number > max_line_number {
                max_line_number = entry.line_number;
            }

            debug_entries.push(entry);
        }

        Ok(DebugSection::new(
            range_size,
            max_line_number,
            debug_entries,
        ))
    }

    pub fn find(&self, addr: u32) -> Option<(&DebugEntry, (u32, u32))> {
        for entry in self.debug_entries.iter() {
            for (range_start, range_end) in entry.ranges.iter() {
                if addr >= *range_start && addr <= *range_end {
                    return Some((entry, (*range_start, *range_end)));
                }
            }
        }

        None
    }

    pub fn get_range_bytes(&self) -> u8 {
        self.range_size
    }

    pub fn get_debug_entries(&self) -> &Vec<DebugEntry> {
        &self.debug_entries
    }

    pub fn get_max_line_number(&self) -> u16 {
        self.max_line_number
    }

    pub fn dump(&self) -> Result<(), Box<dyn Error>> {

        let mut term = Terminal::new(ColorSpec::new());

        term.writeln(&String::from("\nDebug section:"))?;

        for entry in self.debug_entries.iter() {
            term.write(&format!(
                "  Line {}, {} range{}:",
                entry.line_number,
                entry.number_ranges,
                if entry.number_ranges > 1 { "s" } else { "" }
            ))?;

            for (range_start, range_end) in entry.ranges.iter() {

                term.write(&format!(" [{:06x}, {:06x}]", range_start, range_end))?;
            }

            term.writeln(&String::from(""))?;
        }

        Ok(())
    }
}

pub struct DebugEntry {
    pub line_number: u16,
    pub number_ranges: usize,
    pub ranges: Vec<(u32, u32)>,
}

impl DebugEntry {
    pub fn new(line_number: u16, number_ranges: usize, ranges: Vec<(u32, u32)>) -> DebugEntry {
        DebugEntry {
            line_number,
            number_ranges,
            ranges,
        }
    }

    pub fn read(reader: &mut KSMFileReader) -> Result<DebugEntry, Box<dyn Error>> {
        let line_number = reader.read_int16()? as u16;

        let number_ranges = reader.next()? as usize;

        let mut ranges: Vec<(u32, u32)> = Vec::with_capacity(number_ranges);

        for _ in 0..number_ranges {
            let range_start = reader.read_debug_range_address()?;
            let range_end = reader.read_debug_range_address()?;

            ranges.push((range_start, range_end));
        }

        Ok(DebugEntry::new(line_number, number_ranges, ranges))
    }
}

/// Represents an argument section in a KSM file
pub struct ArgumentSection {
    addr_bytes: u8,
    argument_list: Vec<Argument>,
    index_to_addr: HashMap<usize, u32>,
    addr_to_index: HashMap<u32, usize>,
}

impl ArgumentSection {
    pub fn new(addr_bytes: u8, argument_list: Vec<Argument>) -> ArgumentSection {
        let mut index_to_addr: HashMap<usize, u32> = HashMap::with_capacity(argument_list.len());
        let mut addr_to_index: HashMap<u32, usize> = HashMap::with_capacity(argument_list.len());

        for (index, argument) in argument_list.iter().enumerate() {
            index_to_addr.insert(index, argument.get_address());
            addr_to_index.insert(argument.get_address(), index);
        }

        ArgumentSection {
            addr_bytes,
            argument_list,
            index_to_addr,
            addr_to_index,
        }
    }

    pub fn get_index(&self, addr: u32) -> Result<usize, Box<dyn Error>> {
        match self.addr_to_index.get(&addr) {
            Some(index) => Ok(*index),
            None => Err(format!(
                "Address {} is not a valid argument in the argument section.",
                addr
            )
            .into()),
        }
    }

    pub fn get_addr(&self, index: usize) -> Result<u32, Box<dyn Error>> {
        match self.index_to_addr.get(&index) {
            Some(addr) => Ok(*addr),
            None => Err(format!("Argument {} not found in the argument section.", index).into()),
        }
    }

    pub fn get_argument(&self, addr: u32) -> &Argument {
        self.argument_list
            .get(*self.addr_to_index.get(&addr).unwrap())
            .unwrap()
    }

    pub fn get_addr_bytes(&self) -> u8 {
        self.addr_bytes
    }

    pub fn get_argument_list(&self) -> &Vec<Argument> {
        &self.argument_list
    }

    pub fn read(reader: &mut KSMFileReader) -> Result<ArgumentSection, Box<dyn Error>> {
        // The number if bytes required to represent an address into this argument section
        let addr_bytes: u8 = reader.next()?;

        reader.set_address_bytes(addr_bytes);

        let mut argument_list: Vec<Argument> = Vec::new();

        while reader.peek()? != b'%' {
            argument_list.push(Argument::read(reader)?);
        }

        Ok(ArgumentSection::new(addr_bytes, argument_list))
    }

    pub fn dump(&self) -> Result<(), Box<dyn Error>> {

        let mut term = Terminal::new(ColorSpec::new());

        term.writeln(&format!("\nArgument section {} byte indexing:", self.addr_bytes))?;

        term.writeln(&format!("  {:<12}{:<24}{}", "Type", "Value", "Index"))?;

        let mut variable_color = ColorSpec::new();
        variable_color.set_fg(Some(VARIABLE_COLOR));

        let mut type_color = ColorSpec::new();
        type_color.set_fg(Some(TYPE_COLOR));

        for argument in self.argument_list.iter() {
            term.write_colored(&format!("  {:<12}", argument.get_type_str()), &type_color)?;

            if argument.is_variable() {
                term.write_colored(&format!("{:<24}", argument.get_repr()), &variable_color)?;
            } else {
                term.write(&format!("{:<24}", argument.get_repr()))?;
            }

            term.writeln(&format!("{:>}", argument.get_address()))?;
        }

        Ok(())
    }
}
