use std::{error::Error};

use kerbalobjects::{KOFile, KOSValue};

use crate::{CLIConfig, Terminal, Value, Argument, ADDRESS_COLOR, LINE_COLOR, MNEMONIC_COLOR, TYPE_COLOR, VARIABLE_COLOR};
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

        // If we want to display the header
        if config.file_headers || config.full_contents || config.all_headers {
            self.dump_koheader()?;
        }

        // If we want to display the section header table
        if config.section_headers || config.full_contents || config.all_headers {
            self.dump_section_table()?;
        }

        // If we want to display the symbol string section
        if config.stab || config.full_contents {
            self.dump_string_table()?;
        }

        // If we want to display the symbol data section
        if config.data || config.full_contents {
            self.dump_symbol_data()?;
        }

        // If we want to display the symbol table
        if config.syms || config.full_contents {
            self.dump_symbol_table()?;
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

    /// Dumps all of the section headers in the section header table
    fn dump_section_table(&mut self) -> Result<(), Box<dyn Error>> {

        self.term.writeln("\nSections:")?;

        self.term.writeln(&format!("{:<7}{:<16}{:<12}{:<12}{:<12}", "Index", "Name", "Type", "Size", "Offset"))?;

        // Loop through each header and display it's information
        for (index, header) in self.kofile.get_header_table().get_headers().iter().enumerate() {
            self.term.write(&format!("{:<7}", index))?;
            self.term.write(&format!("{:<16}", header.name()))?;
            self.term.write(&format!("{:<12}", format!("{:?}", header.get_type())))?;
            self.term.write(&format!("{:<12}", header.size()))?;
            self.term.writeln(&format!("{:<12}\n", header.offset()))?;
        }

        Ok(())
    }

    /// Dumps the symbol string table in the KO file
    fn dump_string_table(&mut self) -> Result<(), Box<dyn Error>> {

        self.term.writeln("\nSymbol String Table:")?;

        // We will use the address color for the indexes
        let mut address_color = ColorSpec::new();
        address_color.set_fg(Some(ADDRESS_COLOR));

        // We will use the variable color for the strings themselves
        let mut variable_color = ColorSpec::new();
        variable_color.set_fg(Some(VARIABLE_COLOR));

        // We will need to keep track of the index into the string table ourselves
        let mut str_idx = 0;

        // Loop through each string in the string table
        for (i, s) in self.kofile.get_symstrtab().get_strings().iter().enumerate() {

            // Add the string's length to the count, adding +1 to account for the null byte
            str_idx += 1 + s.len();

            // Skip the first string, as it will always be null
            if i == 0 {
                continue;
            }

            // Print the index into the string table of the string
            self.term.write_colored(&format!("  [{:<6}]", str_idx), &address_color)?;

            // Print the actual string itself
            self.term.writeln_colored(&format!("  {}", s), &variable_color)?;
        }

        Ok(())
    }

    /// Dumps the symbol data section containing all kOS values used by the file
    fn dump_symbol_data(&mut self) -> Result<(), Box<dyn Error>> {

        self.term.writeln("\nSymbol Data Table:")?;

        self.term.writeln(&format!("  {:<12}{:<24}{}", "Type", "Value", "Index"))?;

        let mut variable_color = ColorSpec::new();
        variable_color.set_fg(Some(VARIABLE_COLOR));

        let mut type_color = ColorSpec::new();
        type_color.set_fg(Some(TYPE_COLOR));

        // We will need to keep track of the index into the data table ourselves
        let mut data_idx = 0;

        // Loop through each kos value
        for kos_value in self.kofile.get_symdata().get_values().iter() {

            // The best way to get this into printable form is to turn it into an argument
            let type_num = KOFileDebug::kosvalue_to_type(kos_value);
            let value = KOFileDebug::kosvalue_to_value(kos_value);
            let argument = Argument::new(type_num, data_idx, value);

            // Increment the index with the size of the value
            data_idx += kos_value.size() as u32;

            // All of this is the exact same as when dumping an argument section
            self.term.write_colored(&format!("  {:<12}", argument.get_type_str()), &type_color)?;

            if argument.is_variable() {
                self.term.write_colored(&format!("{:<24}", argument.get_repr()), &variable_color)?;
            } else {
                self.term.write(&format!("{:<24}", argument.get_repr()))?;
            }

            self.term.writeln(&format!("{:>}", argument.get_address()))?;
        }

        Ok(())
    }

    /// Dumps all of the symbols in the symbol table
    fn dump_symbol_table(&mut self) -> Result<(), Box<dyn Error>> {

        self.term.writeln("\nSymbol Table:")?;

        // Loop through each symbol in the symbol table
        for (i, symbol) in self.kofile.get_symtab().get_symbols().iter().enumerate() {

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