use flate2::read::GzDecoder;
use std::error::Error;
use std::io::Read;

use termcolor::ColorSpec;

mod sections;
pub use sections::{ArgumentSection, CodeSection, DebugSection, DebugEntry, SectionType};

use crate::{CLIConfig, Terminal, Value};

pub struct KSMFile {
    argument_section: ArgumentSection,
    code_sections: Vec<CodeSection>,
    debug_section: DebugSection,
}

impl KSMFile {
    pub fn new(
        argument_section: ArgumentSection,
        code_sections: Vec<CodeSection>,
        debug_section: DebugSection,
    ) -> KSMFile {
        KSMFile {
            argument_section,
            code_sections,
            debug_section,
        }
    }

    pub fn read(reader: &mut KSMFileReader) -> Result<KSMFile, Box<dyn Error>> {
        let argument_section: ArgumentSection;
        let mut code_sections: Vec<CodeSection> = Vec::new();
        let debug_section: DebugSection;

        // Removes the file magic numbers
        reader.pop(4)?;

        if reader.next()? != b'%' || reader.next()? != b'A' {
            return Err("Argument section expected".into());
        }

        argument_section = ArgumentSection::read(reader)?;

        let mut peeked = reader.peek_count(2)?;

        while !(peeked[0] == b'%' && peeked[1] == b'D') {
            code_sections.push(CodeSection::read(reader)?);

            peeked = reader.peek_count(2)?;
        }

        debug_section = DebugSection::read(reader)?;

        Ok(KSMFile::new(argument_section, code_sections, debug_section))
    }

    pub fn get_argument_section(&self) -> &ArgumentSection {
        &self.argument_section
    }

    pub fn get_debug_section(&self) -> &DebugSection {
        &self.debug_section
    }

    pub fn get_code_sections(&self) -> &Vec<CodeSection> {
        &self.code_sections
    }

    pub fn get_compiler(&self) -> String {
        // Tests the first argment of the KSM file argument section
        match self.argument_section.get_argument_list().get(0) {
            Some(arg) => match arg.get_value() {
                Value::String(s) => {
                    // If it is either a label that is used for reset or a KS formatted function name
                    if s.starts_with("@") || s.contains("`") {
                        String::from("Compiled using internal kOS compiler.")
                    } else {
                        format!("{}", s)
                    }
                }
                _ => String::from("Unknown compiler."),
            },
            None => String::from("Unknown compiler. Not enough data."),
        }
    }

    pub fn dump(&self, config: &CLIConfig) -> Result<(), Box<dyn Error>> {

        let mut term = Terminal::new(ColorSpec::new());

        if config.info {
            term.writeln(&String::from("\nKSM File Info:"))?;

            term.writeln(&format!("  {}", self.get_compiler()))?;
        }

        if config.full_contents || config.argument_section {
            self.argument_section.dump()?;
        }

        if config.full_contents || config.disassemble {
            let mut offset = 0;
            let mut instruction_index = 1;

            for section in self.code_sections.iter() {
                section.dump(
                    offset,
                    instruction_index,
                    !config.show_no_labels,
                    !config.show_no_raw_insn,
                    config.line_numbers,
                    &self.argument_section,
                    &self.debug_section,
                )?;

                offset += section.size();
                instruction_index += section.number_real_instructions();
            }
        } else if config.disassemble_symbol {
            let mut offset = 0;
            let mut instruction_index = 1;

            for section in self.code_sections.iter() {
                // Checks if the section contains the symbol that was speciifed by the command line argument
                if section.contains(&config.disassemble_symbol_value, &self.argument_section)?
                    || (section.get_type() == SectionType::MAIN
                        && config.disassemble_symbol_value.eq_ignore_ascii_case("main"))
                {
                    section.dump(
                        offset,
                        instruction_index,
                        !config.show_no_labels,
                        !config.show_no_raw_insn,
                        config.line_numbers,
                        &self.argument_section,
                        &self.debug_section,
                    )?;

                    break;
                }

                offset += section.size();
                instruction_index += section.number_real_instructions();
            }
        }

        if config.full_contents {
            self.debug_section.dump()?;
        }

        Ok(())
    }
}

pub struct KSMFileReader {
    current_index: usize,
    contents: Vec<u8>,
    arg_address_bytes: u8,
    debug_range_bytes: u8,
}

impl KSMFileReader {
    /// Creates and returns a new instance of KSMFileReader
    pub fn new(raw_contents: Vec<u8>) -> Result<KSMFileReader, Box<dyn Error>> {
        // Create a new GZip decoder so that we can read the real contents of the file
        let mut decoder = GzDecoder::new(&raw_contents[..]);
        let mut decompressed: Vec<u8> = Vec::new();

        // Read all of the decompressed bytes
        decoder.read_to_end(&mut decompressed)?;

        // Return a new instance with the current index at 0
        Ok(KSMFileReader {
            current_index: 0,
            contents: decompressed,
            arg_address_bytes: 0,
            debug_range_bytes: 0,
        })
    }

    pub fn set_address_bytes(&mut self, address_bytes: u8) {
        self.arg_address_bytes = address_bytes;
    }

    pub fn get_address_bytes(&self) -> u8 {
        self.arg_address_bytes
    }

    pub fn set_range_bytes(&mut self, range_bytes: u8) {
        self.debug_range_bytes = range_bytes;
    }

    pub fn get_range_bytes(&self) -> u8 {
        self.debug_range_bytes
    }

    /// Returns the current index of the reader into the byte vector
    pub fn get_current_index(&self) -> usize {
        self.current_index
    }

    pub fn eof(&self) -> bool {
        self.current_index >= (self.contents.len() - 1)
    }

    /// Simply discards the next byte from the contents vector, and advances the current index
    pub fn pop(&mut self, bytes: usize) -> Result<(), Box<dyn Error>> {
        self.current_index += bytes;

        if self.current_index <= self.contents.len() {
            Ok(())
        } else {
            Err("Unexpected EOF reached".into())
        }
    }

    /// Reads the next byte from the contents vector and returns it if there is one
    pub fn next(&mut self) -> Result<u8, Box<dyn Error>> {
        // Increment the index
        self.current_index += 1;

        // Return the next byte or throw an error
        match self.contents.get(self.current_index - 1) {
            Some(byte) => Ok(*byte),
            None => Err("Unexpected EOF reached".into()),
        }
    }

    /// Reads count bytes from the contents and returns a vector of them
    pub fn next_count(&mut self, count: usize) -> Result<Vec<u8>, Box<dyn Error>> {
        let mut read_bytes: Vec<u8> = Vec::with_capacity(count);

        for _ in 0..count {
            read_bytes.push(self.next()?);
        }

        Ok(read_bytes)
    }

    /// Peeks one byte from the contents
    pub fn peek(&self) -> Result<u8, Box<dyn Error>> {
        // Return the next byte or throw an error
        match self.contents.get(self.current_index) {
            Some(byte) => Ok(*byte),
            None => Err("Unexpected EOF reached".into()),
        }
    }

    /// Peeks count bytes from the contents and returns a vector of them
    pub fn peek_count(&mut self, count: usize) -> Result<Vec<u8>, Box<dyn Error>> {
        let original_index = self.current_index;
        let mut peeked: Vec<u8> = Vec::with_capacity(count);

        for _ in 0..count {
            peeked.push(self.next()?);
        }

        self.current_index = original_index;

        Ok(peeked)
    }

    pub fn read_bytes_into_u32(&mut self, bytes: u8) -> Result<u32, Box<dyn Error>> {
        Ok(match bytes {
            0 => panic!("One should never try to read 0 bytes."),
            1 => self.next()? as u32,
            2 => self.read_uint16_be()? as u32,
            3 => (self.read_uint16_be()? as u32 * 0x010000) + self.next()? as u32,
            4 => self.read_uint32_be()? as u32,
            _ => {
                return Err(
                    "Currently reading more than 4 bytes at a time into an address is unsupported"
                        .into(),
                )
            }
        })
    }

    pub fn read_argument_address(&mut self) -> Result<u32, Box<dyn Error>> {
        self.read_bytes_into_u32(self.arg_address_bytes)
    }

    pub fn read_debug_range_address(&mut self) -> Result<u32, Box<dyn Error>> {
        self.read_bytes_into_u32(self.debug_range_bytes)
    }

    pub fn read_boolean(&mut self) -> Result<bool, Box<dyn Error>> {
        Ok(self.next()? != 0u8)
    }

    pub fn read_byte(&mut self) -> Result<i8, Box<dyn Error>> {
        Ok(self.next()? as i8)
    }

    pub fn read_int16(&mut self) -> Result<i16, Box<dyn Error>> {
        let mut arr: [u8; 2] = [0u8; 2];

        for i in 0..2 {
            arr[i] = self.next()?;
        }

        Ok(i16::from_le_bytes(arr))
    }

    pub fn read_uint16(&mut self) -> Result<u16, Box<dyn Error>> {
        let mut arr: [u8; 2] = [0u8; 2];

        for i in 0..2 {
            arr[i] = self.next()?;
        }

        Ok(u16::from_le_bytes(arr))
    }

    pub fn read_uint16_be(&mut self) -> Result<u16, Box<dyn Error>> {
        let mut arr: [u8; 2] = [0u8; 2];

        for i in 0..2 {
            arr[i] = self.next()?;
        }

        Ok(u16::from_be_bytes(arr))
    }

    pub fn read_int32(&mut self) -> Result<i32, Box<dyn Error>> {
        let mut arr: [u8; 4] = [0u8; 4];

        for i in 0..4 {
            arr[i] = self.next()?;
        }

        Ok(i32::from_le_bytes(arr))
    }

    pub fn read_uint32(&mut self) -> Result<u32, Box<dyn Error>> {
        let mut arr: [u8; 4] = [0u8; 4];

        for i in 0..4 {
            arr[i] = self.next()?;
        }

        Ok(u32::from_le_bytes(arr))
    }

    pub fn read_uint32_be(&mut self) -> Result<u32, Box<dyn Error>> {
        let mut arr: [u8; 4] = [0u8; 4];

        for i in 0..4 {
            arr[i] = self.next()?;
        }

        Ok(u32::from_be_bytes(arr))
    }

    pub fn read_float(&mut self) -> Result<f32, Box<dyn Error>> {
        let mut arr: [u8; 4] = [0u8; 4];

        for i in 0..4 {
            arr[i] = self.next()?;
        }

        Ok(f32::from_le_bytes(arr))
    }

    pub fn read_double(&mut self) -> Result<f64, Box<dyn Error>> {
        let mut arr: [u8; 8] = [0u8; 8];

        for i in 0..8 {
            arr[i] = self.next()?;
        }

        Ok(f64::from_le_bytes(arr))
    }

    pub fn read_string(&mut self) -> Result<String, Box<dyn Error>> {
        let len = self.next()? as usize;

        let mut internal = String::with_capacity(len);

        for _ in 0..len {
            internal.push(self.next()? as char);
        }

        Ok(internal)
    }
}
