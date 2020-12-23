use std::{error::Error};

use kerbalobjects::{KOFile, KOSValue, SymbolType};

use crate::{CLIConfig, Terminal, Value, Argument, Instr, PURPLE_COLOR, DARK_RED_COLOR, GREEN_COLOR, LIGHT_RED_COLOR};
use termcolor::ColorSpec;

pub struct KOFileDebug {
    kofile: KOFile,
    term: Terminal
}

impl KOFileDebug {
    /// Creates a new KO debug object for a KOFile
    pub fn new(kofile: KOFile) -> KOFileDebug {
        KOFileDebug {
            kofile,
            term: Terminal::new(ColorSpec::new()),
        }
    }

    /// Dumps the KO file using the specified settings
    pub fn dump(&mut self, config: &CLIConfig) -> Result<(), Box<dyn Error>> {

        // If we want to display compiler/assembler info
        if config.info {
            self.dump_info()?;
        }

        // If we want to display the header
        if config.file_headers || config.all_headers {
            self.dump_koheader()?;
        }

        // If we want to display the section header table
        if config.section_headers || config.all_headers {
            self.dump_section_table()?;
        }

        // If we want to display the string sections
        if config.stabs || config.full_contents {
            self.dump_string_tables()?;
        }

        // If we want to display the symbol data section
        if config.data || config.full_contents {
            self.dump_symbol_data()?;
        }

        // If we want to display the symbol table
        if config.syms || config.full_contents {
            self.dump_symbol_table()?;
        }

        // If we want to display the fully disassembled file
        if config.disassemble || config.full_contents {
            self.dump_rel_sections(config)?;
        }

        // If we are searching for a specific symbol and disassembling
        if config.disassemble_symbol {
            self.dump_rel_symbol(&config.disassemble_symbol_value, config)?;
        }

        Ok(())
    }

    /// Dumps the contents of the KO file header
    fn dump_koheader(&mut self) -> Result<(), Box<dyn Error>> {

        self.term.writeln("\nFile header:")?;

        self.term.writeln(&format!("\tFile length: {} bytes", self.kofile.length()))?;

        self.term.writeln(&format!("\tVersion: {}", self.kofile.version()))?;

        self.term.writeln(&format!("\tNumber of sections: {}", self.kofile.num_sections()))?;

        Ok(())
    }

    /// Dumps the only string contained in the comment section if there is one
    fn dump_info(&mut self) -> Result<(), Box<dyn Error>> {

        self.term.writeln(&String::from("\nKO File Info:"))?;

        // This needs to be an option because KO files are not required to have comment sections
        let mut comment_section = None;

        // Loop through each string table in the file
        for string_table in self.kofile.get_string_tables() {
            // Check if there even is a comment section
            if string_table.name() == ".comment" {
                // If there is, store the reference to it
                comment_section = Some(string_table);
            }
        }

        match comment_section {
            Some(comment_section) => {
                // If there is one, check if it is populated
                match comment_section.get(1) {
                    Ok(comment) => {
                        // If it is populated, the comment will be the first and only string
                        self.term.writeln(&format!("  {}", comment))?;
                    },
                    Err(_) => {
                        self.term.writeln("  Comment section empty.")?;
                    }
                }
            },
            None => {
                self.term.writeln("  None")?;
            }
        }

        Ok(())
    }

    /// Dumps all of the section headers in the section header table
    fn dump_section_table(&mut self) -> Result<(), Box<dyn Error>> {

        let mut name_color = ColorSpec::new();
        name_color.set_fg(Some(LIGHT_RED_COLOR));
        let mut type_color = ColorSpec::new();
        type_color.set_fg(Some(GREEN_COLOR));
        let mut number_color = ColorSpec::new();
        number_color.set_fg(Some(PURPLE_COLOR));

        self.term.writeln("\nSections:")?;

        self.term.writeln(&format!("{:<7}{:<16}{:<12}{:<12}{:<12}", "Index", "Name", "Type", "Size", "Offset"))?;

        // Loop through each header and display it's information
        for (index, header) in self.kofile.get_header_table().get_headers().iter().enumerate() {
            self.term.write(&format!("{:<7}", index))?;
            self.term.write_colored(&format!("{:<16}", header.name()), &name_color)?;
            self.term.write_colored(&format!("{:<12}", format!("{:?}", header.get_type())), &type_color)?;
            self.term.write_colored(&format!("{:<12}", header.size()), &number_color)?;
            self.term.writeln_colored(&format!("{:<12}\n", header.offset()), &number_color)?;
        }

        Ok(())
    }

    /// Dumps all string tables in the KO file
    fn dump_string_tables(&mut self) -> Result<(), Box<dyn Error>> {

        self.term.writeln("\nString tables:")?;

        // We will use the purple color for the indexes
        let mut index_color = ColorSpec::new();
        index_color.set_fg(Some(PURPLE_COLOR));

        // We will use the red color for the strings themselves
        let mut str_color = ColorSpec::new();
        str_color.set_fg(Some(LIGHT_RED_COLOR));

        // We need to make our own vector to keep all of the string tables
        let mut all_string_tables = vec![ self.kofile.get_symstrtab() ];

        // Add each string table to the vector
        for table in self.kofile.get_string_tables().iter() {
            all_string_tables.push(table);
        }

        // Loop through each table
        for table in all_string_tables {

            // Print the name of the table
            self.term.writeln(&format!("Table {}:", table.name()))?;

            // We will need to keep track of the index into the string table ourselves
            let mut str_idx = 0;

            // Loop through each string in the string table
            for (i, s) in table.get_strings().iter().enumerate() {

                // Skip the first string, as it will always be null
                if i == 0 {
                    str_idx += 1;
                    continue;
                }

                // Print the index into the string table of the string
                self.term.write("  [")?;
                self.term.write_colored(&format!("{:>6}", str_idx), &index_color)?;
                self.term.write("]")?;

                // Print the actual string itself
                self.term.writeln_colored(&format!("  {}", s), &str_color)?;

                // Add the string's length to the count, adding +1 to account for the null byte
                str_idx += 1 + s.len();
            }
        }


        Ok(())
    }

    /// Dumps the symbol data section containing all kOS values used by the file
    fn dump_symbol_data(&mut self) -> Result<(), Box<dyn Error>> {

        self.term.writeln("\nSymbol Data Table:")?;

        self.term.writeln(&format!("{:<12}{:<12}{}", "Index", "Type", "Value"))?;

        let mut variable_color = ColorSpec::new();
        variable_color.set_fg(Some(LIGHT_RED_COLOR));

        let mut type_color = ColorSpec::new();
        type_color.set_fg(Some(GREEN_COLOR));

        // We will need to keep track of the index into the data table ourselves
        let mut data_idx = 0;

        // Loop through each kos value
        for (i, kos_value) in self.kofile.get_symdata().get_values().iter().enumerate() {

            // The best way to get this into printable form is to turn it into an argument
            let type_num = KOFileDebug::kosvalue_to_type(kos_value);
            let value = KOFileDebug::kosvalue_to_value(kos_value);
            let argument = Argument::new(type_num, data_idx, value);

            // Increment the index with the size of the value
            data_idx += kos_value.size() as u32;

            self.term.write(&format!("  {:<10x}", i))?;

            self.term.write_colored(&format!("{:<12}", argument.get_type_str()), &type_color)?;

            if argument.is_variable() {
                self.term.writeln_colored(&format!("{}", argument.get_repr()), &variable_color)?;
            } else {
                self.term.writeln(&format!("{}", argument.get_repr()))?;
            }
        }

        Ok(())
    }

    /// Dumps all of the symbols in the symbol table
    fn dump_symbol_table(&mut self) -> Result<(), Box<dyn Error>> {

        // Set up our colors
        let mut name_color = ColorSpec::new();
        name_color.set_fg(Some(LIGHT_RED_COLOR));
        let mut number_color = ColorSpec::new();
        number_color.set_fg(Some(PURPLE_COLOR));
        let mut type_color = ColorSpec::new();
        type_color.set_fg(Some(GREEN_COLOR));

        self.term.writeln("\nSymbol Table:")?;

        self.term.writeln(&format!("{:<12}{:<10}{:<8}{:<8}{:<10}{}", "Name", "Value", "Size", "Info", "Type", "Section"))?;

        // Loop through each symbol in the symbol table
        for symbol in self.kofile.get_symtab().get_symbols().iter() {
            self.term.write_colored(&format!("{:<12}", symbol.name()), &name_color)?;
            self.term.write_colored(&format!("{:0>8x}  ", symbol.get_value_index()), &number_color)?;
            self.term.write_colored(&format!("{:0>4x}    ", symbol.size()), &number_color)?;
            self.term.write_colored(&format!("{:<8}", format!("{:?}", symbol.get_info())), &type_color)?;
            self.term.write_colored(&format!("{:<10}", format!("{:?}", symbol.get_type())), &type_color)?;
            self.term.writeln(&format!("{}", symbol.get_section_index()))?;
        }

        Ok(())
    }

    /// Dumps a specific rel section if it contains the symbol provided
    pub fn dump_rel_symbol(&mut self, symbol: &str, config: &CLIConfig) -> Result<(), Box<dyn Error>> {

        // Loop through each REL section in the file
        for rel_section_index in 0..self.kofile.get_code_sections().len() {
            // Check if it contains the symbol
            if self.rel_section_contains(rel_section_index, symbol)? {
                // If it does, dump it
                self.dump_rel_section(rel_section_index, config)?;
            }
        }

        Ok(())
    }

    /// Dumps a disassembled output of each REL section in the KO file
    pub fn dump_rel_sections(&mut self, config: &CLIConfig) -> Result<(), Box<dyn Error>> {

        // Loop through each REL section in the file
        for rel_section_index in 0..self.kofile.get_code_sections().len() {
            self.dump_rel_section(rel_section_index, config)?;
        }

        Ok(())
    }
 
    /// Returns true if a certain rel section contains a certain symbol
    /// This will also return true of this section's name is the symbol
    pub fn rel_section_contains(&self, rel_section_index: usize, symbol: &str) -> Result<bool, Box<dyn Error>> {

        let rel_section = self.kofile.get_code_sections().get(rel_section_index).unwrap();

        // Check if this section's name is the symbol
        if rel_section.name() == symbol {
            Ok(true)
        } else {
            let instr_list = rel_section.get_instructions();

            // Now we have to loop through each instruction in the section
            for instr in instr_list.iter() {
                for op in instr.get_operands().iter() {
                    // Get the symbol
                    let sym = self.kofile.get_symtab().get(*op as usize)?;

                    // Check if the name is the same
                    if sym.name() == symbol {
                        // If it is, then it is contained
                        return Ok(true);
                    } else {
                        // If not, check the value

                        // If it isn't a NOTYPE, then that is false for this one
                        if sym.get_type() == SymbolType::NOTYPE {
                            // If it is, get the KOSValue
                            let kos_value = self.kofile.get_symdata().get(sym.get_value_index())?;

                            // If it is a string or stringvalue, we have a chance
                            match kos_value {
                                KOSValue::STRING(s) | KOSValue::STRINGVALUE(s) => {
                                    // Check it
                                    if s == symbol {
                                        return Ok(true);
                                    }
                                },
                                _ => {}
                            }
                        }
                    }
                }
            }

            Ok(false)
        }        
    }

    /// Dumps a single rel section from the KO file using the rel section's index
    pub fn dump_rel_section(&mut self, rel_section_index: usize, config: &CLIConfig) -> Result<(), Box<dyn Error>> {
        // Set up our colors
        let mut index_color = ColorSpec::new();
        index_color.set_fg(Some(PURPLE_COLOR));
        let mut mnemonic_color = ColorSpec::new();
        mnemonic_color.set_fg(Some(DARK_RED_COLOR));
        let mut variable_color = ColorSpec::new();
        variable_color.set_fg(Some(LIGHT_RED_COLOR));
        let mut type_color = ColorSpec::new();
        type_color.set_fg(Some(GREEN_COLOR));

        let rel_section = self.kofile.get_code_sections().get(rel_section_index).unwrap();

        let instr_list = rel_section.get_instructions();

        // Write the name of the section
        self.term.writeln(&format!("\nRelocatable section {}:", rel_section.name()))?;

        // Loop through each instruction
        for (i, instr) in instr_list.iter().enumerate() {
            // We need to do this in order to get the mnemonic of the instruction...
            let storage_instr = Instr::new(instr.get_opcode(), 0, 0, vec![]);

            // Pad the output a little
            self.term.write("  ")?;
            
            // If we want to show "labels"
            if !config.show_no_labels {
                // Because kOS labels start with instruction @0001, we should start from 1 here too
                self.term.write_colored(&format!("{:0>8x} ", i+1), &index_color)?;
            }

            // If we want to show the raw instructions
            if !config.show_no_raw_insn {
                // Write the opcode
                self.term.write(&format!("{:0>2x} ", instr.get_opcode()))?;

                // Loop through each operand and write it
                for op in instr.get_operands() {
                    self.term.write(&format!("{:0>4x} ", *op))?;
                }

                // We will need to print empty spaces for the operands we don't have
                for _ in 0..(2-instr.get_operands().len()) {
                    self.term.write(&format!("     "))?;
                }
            }

            // Write out the mnemonic
            self.term.write_colored(&format!(" {:<4}", storage_instr.get_mnemonic()), &mnemonic_color)?;

            // Write out each operand
            for (j, op) in instr.get_operands().iter().enumerate() {
                // Get the symbol that this operand points to
                let sym = match self.kofile.get_symtab().get(*op as usize) {
                    Ok(sym) => sym,
                    Err(_) => {
                        return Err(format!("Instruction references undefined symbol {}", op).into());
                    }
                };

                // Based on the type of the symbol we want to do different things
                match sym.get_type() {
                    // This is the symbol type used by regular KOSValues
                    SymbolType::NOTYPE => {
                        // Get the value
                        let kos_value = self.kofile.get_symdata().get(sym.get_value_index())?;

                        // The best way to get this into printable form is to turn it into an argument
                        let type_num = KOFileDebug::kosvalue_to_type(kos_value);
                        let value = KOFileDebug::kosvalue_to_value(kos_value);
                        let argument = Argument::new(type_num, 0, value);

                        if argument.is_variable() {
                            self.term.write_colored(&format!(" {}", argument.get_repr()), &variable_color)?;
                        } else {
                            self.term.write(&format!(" {}", argument.get_repr()))?;
                        }
                    },
                    SymbolType::FUNC => {
                        // This is a function so all we need to print is the name
                        self.term.write_colored(&format!(" {}", sym.name()), &type_color)?;
                    },
                    _ => {
                        return Err("KDump is currently unable to print symbols other than NOTYPE and FUNC".into());
                    }
                }

                // If this isn't the last one
                if j < instr.get_operands().len() - 1 {
                    // Write a comma
                    self.term.write(",")?;
                }
            }

            // Write a newline
            self.term.writeln("")?;
        }

        Ok(())
    }

    /// Converts a KerbalObjects KOSValue into a Value enum
    /// TODO: Use one or the other and I wouldn't need this
    fn kosvalue_to_value(kos_value: &KOSValue) -> Value {
        match kos_value {
            KOSValue::NULL => Value::NULL,
            KOSValue::BOOL(b) => Value::Boolean(*b),
            KOSValue::BYTE(b) => Value::Byte(*b),
            KOSValue::INT16(i) => Value::Int16(*i),
            KOSValue::INT32(i) => Value::Int32(*i),
            KOSValue::FLOAT(f) => Value::Float(*f),
            KOSValue::DOUBLE(d) => Value::Double(*d),
            KOSValue::STRING(s) => Value::String(s.to_owned()),
            KOSValue::ARGMARKER => Value::ARGMARKER,
            KOSValue::SCALARINT(i) => Value::ScalarIntValue(*i),
            KOSValue::SCALARDOUBLE(d) => Value::ScalarDoubleValue(*d),
            KOSValue::BOOLEANVALUE(b) => Value::BooleanValue(*b),
            KOSValue::STRINGVALUE(s) => Value::StringValue(s.to_owned())
        }
    }

    /// Returns the int that corresponds to the type number of this kos value
    fn kosvalue_to_type(kos_value: &KOSValue) -> usize {
        match kos_value {
            KOSValue::NULL => 0,
            KOSValue::BOOL(_) => 1,
            KOSValue::BYTE(_) => 2,
            KOSValue::INT16(_) => 3,
            KOSValue::INT32(_) => 4,
            KOSValue::FLOAT(_) => 5,
            KOSValue::DOUBLE(_) => 6,
            KOSValue::STRING(_) => 7,
            KOSValue::ARGMARKER => 8,
            KOSValue::SCALARINT(_) => 9,
            KOSValue::SCALARDOUBLE(_) => 10,
            KOSValue::BOOLEANVALUE(_) => 11,
            KOSValue::STRINGVALUE(_) => 12,
        }
    }
}