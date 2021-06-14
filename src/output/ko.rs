use kerbalobjects::kofile::sections::DataSection;
use kerbalobjects::kofile::sections::RelSection;
use kerbalobjects::kofile::sections::SectionKind;
use kerbalobjects::kofile::sections::StringTable;
use kerbalobjects::kofile::sections::SymbolTable;
use kerbalobjects::kofile::symbols::KOSymbol;
use kerbalobjects::kofile::KOFile;
use kerbalobjects::KOSValue;
use std::error::Error;
use std::io::Write;
use termcolor::ColorSpec;
use termcolor::StandardStream;
use termcolor::WriteColor;

use crate::CLIConfig;
use crate::DARK_RED_COLOR;
use crate::GREEN_COLOR;
use crate::LIGHT_RED_COLOR;
use crate::PURPLE_COLOR;

use super::DumpResult;

pub struct KOFileDebug {
    kofile: KOFile,
}

impl KOFileDebug {
    pub fn new(kofile: KOFile) -> Self {
        KOFileDebug { kofile }
    }

    pub fn dump(&self, stream: &mut StandardStream, config: &CLIConfig) -> DumpResult {
        let no_color = ColorSpec::new();
        let mut purple = ColorSpec::new();
        purple.set_fg(Some(PURPLE_COLOR));
        let mut light_red = ColorSpec::new();
        light_red.set_fg(Some(LIGHT_RED_COLOR));
        let mut green = ColorSpec::new();
        green.set_fg(Some(GREEN_COLOR));
        let mut dark_red = ColorSpec::new();
        dark_red.set_fg(Some(DARK_RED_COLOR));

        if config.info {
            self.dump_info(stream)?;
        }

        if config.file_headers || config.all_headers {
            self.dump_ko_header(stream)?;
        }

        if config.section_headers || config.all_headers {
            self.dump_section_headers(stream, &no_color, &light_red, &green, &purple)?;
        }

        if config.stabs || config.full_contents {
            self.dump_strtabs(stream, &no_color, &purple, &light_red)?;
        }

        if config.data || config.full_contents {
            self.dump_data(stream, &no_color, &green, &light_red)?;
        }

        if config.syms || config.full_contents {
            self.dump_symbols(
                stream, &no_color, &light_red, &purple, &purple, &green, &green, &no_color,
            )?;
        }

        if config.disassemble || config.full_contents {
            self.dump_rel_sections(
                stream,
                &no_color,
                &purple,
                &dark_red,
                &light_red,
                &green,
                &purple,
                !config.show_no_labels,
                !config.show_no_raw_instr,
            )?;
        }

        if config.disassemble_symbol {
            self.dump_rel_by_symbol(
                stream,
                &config.disassemble_symbol_value,
                &no_color,
                &purple,
                &dark_red,
                &light_red,
                &green,
                &purple,
                !config.show_no_labels,
                !config.show_no_raw_instr,
            )?;
        }

        Ok(())
    }

    fn get_section_name(&self, sh_index: usize) -> Result<&str, Box<dyn Error>> {
        let header = self.kofile.get_header(sh_index).ok_or(format!(
            "Failed to find KO file section header for string table with index {}",
            sh_index
        ))?;

        let name = self.kofile.get_header_name(header).ok_or(format!(
            "Failed to find the string table with index {}'s name in KO file",
            sh_index
        ))?;

        Ok(name)
    }

    fn get_symstrtab(&self) -> Result<&StringTable, Box<dyn Error>> {
        for strtab in self.kofile.str_tabs() {
            let sh_index = strtab.section_index();
            if self.get_section_name(sh_index)? == ".symstrtab" {
                return Ok(strtab);
            }
        }

        Err("Tried to find .symstrtab, none found".into())
    }

    fn get_symtab(&self) -> Result<&SymbolTable, Box<dyn Error>> {
        for symtab in self.kofile.sym_tabs() {
            let sh_index = symtab.section_index();
            if self.get_section_name(sh_index)? == ".symtab" {
                return Ok(symtab);
            }
        }

        Err("Tried to find .symtab, none found".into())
    }

    fn get_symdata(&self) -> Result<&DataSection, Box<dyn Error>> {
        for data_section in self.kofile.data_sections() {
            let sh_index = data_section.section_index();
            if self.get_section_name(sh_index)? == ".data" {
                return Ok(data_section);
            }
        }

        Err("Tried to find .data, none found".into())
    }

    fn dump_rel_by_symbol(
        &self,
        stream: &mut StandardStream,
        symbol_text: &String,
        regular_color: &ColorSpec,
        index_color: &ColorSpec,
        mnemonic_color: &ColorSpec,
        variable_color: &ColorSpec,
        func_color: &ColorSpec,
        section_color: &ColorSpec,
        show_labels: bool,
        show_raw_instr: bool,
    ) -> DumpResult {
        let mut rel_section_found = None;

        let symtab = self.get_symtab()?;
        let symdata = self.get_symdata()?;
        let symstrtab = self.get_symstrtab()?;

        for rel_section in self.kofile.rel_sections() {
            let sh_index = rel_section.section_index();
            let name = self.get_section_name(sh_index)?;

            if name == symbol_text {
                rel_section_found = Some(rel_section);
                break;
            }

            // Loop through each symbol in the section
            for instr in rel_section.instructions() {
                match instr {
                    kerbalobjects::kofile::Instr::ZeroOp(_) => {}
                    kerbalobjects::kofile::Instr::OneOp(_, op1) => {
                        let sym1 = symtab.get(*op1).ok_or(format!(
                            "Tried to find symbol with index {}, but found none.",
                            op1
                        ))?;
                        if KOFileDebug::symbol_matches_str(sym1, symbol_text, symstrtab, symdata)? {
                            rel_section_found = Some(rel_section);
                            break;
                        }
                    }
                    kerbalobjects::kofile::Instr::TwoOp(_, op1, op2) => {
                        let sym1 = symtab.get(*op1).ok_or(format!(
                            "Tried to find symbol with index {}, but found none.",
                            op1
                        ))?;
                        let sym2 = symtab.get(*op2).ok_or(format!(
                            "Tried to find symbol with index {}, but found none.",
                            op2
                        ))?;
                        if KOFileDebug::symbol_matches_str(sym1, symbol_text, symstrtab, symdata)?
                            || KOFileDebug::symbol_matches_str(
                                sym2,
                                symbol_text,
                                symstrtab,
                                symdata,
                            )?
                        {
                            rel_section_found = Some(rel_section);
                            break;
                        }
                    }
                }
            }

            if rel_section_found.is_some() {
                break;
            }
        }

        match rel_section_found {
            Some(section) => {
                self.dump_rel_section(
                    stream,
                    regular_color,
                    index_color,
                    mnemonic_color,
                    variable_color,
                    func_color,
                    section_color,
                    show_labels,
                    show_raw_instr,
                    section,
                )?;
            }
            None => {
                writeln!(stream, "\nNo section found with that symbol.")?;
            }
        }

        Ok(())
    }

    fn symbol_matches_str(
        symbol: &KOSymbol,
        s: &str,
        symstrtab: &StringTable,
        symdata: &DataSection,
    ) -> Result<bool, Box<dyn Error>> {
        let sym_name_idx = symbol.name_idx();
        let sym_name = symstrtab
            .get(sym_name_idx)
            .ok_or("Error getting name of symbol, index invalid")?;

        let sym_str = match symbol.sym_type() {
            kerbalobjects::kofile::symbols::SymType::NoType => {
                let value = symdata
                    .get(symbol.value_idx())
                    .ok_or("Value referenced by symbol does not exist")?;

                match value {
                    KOSValue::StringValue(v) | KOSValue::String(v) => v,
                    _ => "",
                }
            }
            _ => "",
        };

        Ok(sym_name.contains(s) || sym_str.contains(s))
    }

    fn dump_rel_sections(
        &self,
        stream: &mut StandardStream,
        regular_color: &ColorSpec,
        index_color: &ColorSpec,
        mnemonic_color: &ColorSpec,
        variable_color: &ColorSpec,
        func_color: &ColorSpec,
        section_color: &ColorSpec,
        show_labels: bool,
        show_raw_instr: bool,
    ) -> DumpResult {
        for rel_section in self.kofile.rel_sections() {
            self.dump_rel_section(
                stream,
                regular_color,
                index_color,
                mnemonic_color,
                variable_color,
                func_color,
                section_color,
                show_labels,
                show_raw_instr,
                rel_section,
            )?;
        }

        Ok(())
    }

    fn dump_rel_section(
        &self,
        stream: &mut StandardStream,
        regular_color: &ColorSpec,
        index_color: &ColorSpec,
        mnemonic_color: &ColorSpec,
        variable_color: &ColorSpec,
        func_color: &ColorSpec,
        section_color: &ColorSpec,
        show_labels: bool,
        show_raw_instr: bool,
        rel_section: &RelSection,
    ) -> DumpResult {
        stream.set_color(regular_color)?;

        let sh_index = rel_section.section_index();

        let name = self.get_section_name(sh_index)?;

        let symdata = self.get_symdata()?;

        let symstrtab = self.get_symstrtab()?;

        writeln!(stream, "\nRelocatable section {}:", name)?;

        for (i, instr) in rel_section.instructions().enumerate() {
            write!(stream, "  ")?;

            let instr_opcode;
            let instr_mnemonic: &str;

            if show_labels {
                stream.set_color(index_color)?;
                write!(stream, "{:0>8x} ", i + 1)?;
                stream.set_color(regular_color)?;
            }

            if show_raw_instr {
                match instr {
                    kerbalobjects::kofile::Instr::ZeroOp(opcode) => {
                        write!(stream, "{:0>2x} {:<4} {:<4} ", u8::from(*opcode), "", "")?;
                        instr_opcode = *opcode;
                    }
                    kerbalobjects::kofile::Instr::OneOp(opcode, op1) => {
                        write!(stream, "{:0>2x} {:0>4x} {:<4} ", u8::from(*opcode), op1, "")?;
                        instr_opcode = *opcode;
                    }
                    kerbalobjects::kofile::Instr::TwoOp(opcode, op1, op2) => {
                        write!(
                            stream,
                            "{:0>2x} {:0>4x} {:0>4x} ",
                            u8::from(*opcode),
                            op1,
                            op2
                        )?;
                        instr_opcode = *opcode;
                    }
                }
            } else {
                instr_opcode = match instr {
                    kerbalobjects::kofile::Instr::ZeroOp(opcode) => *opcode,
                    kerbalobjects::kofile::Instr::OneOp(opcode, _) => *opcode,
                    kerbalobjects::kofile::Instr::TwoOp(opcode, _, _) => *opcode,
                }
            }

            instr_mnemonic = instr_opcode.into();

            stream.set_color(mnemonic_color)?;
            write!(stream, " {:<5}", instr_mnemonic)?;
            stream.set_color(regular_color)?;

            match instr {
                kerbalobjects::kofile::Instr::ZeroOp(_) => {}
                kerbalobjects::kofile::Instr::OneOp(_, op1) => {
                    let sym1 = self.symbol_from_operand(*op1)?;

                    KOFileDebug::write_symbol(
                        stream,
                        sym1,
                        symstrtab,
                        symdata,
                        regular_color,
                        variable_color,
                        func_color,
                        section_color,
                    )?;
                }
                kerbalobjects::kofile::Instr::TwoOp(_, op1, op2) => {
                    let sym1 = self.symbol_from_operand(*op1)?;
                    let sym2 = self.symbol_from_operand(*op2)?;

                    KOFileDebug::write_symbol(
                        stream,
                        sym1,
                        symstrtab,
                        symdata,
                        regular_color,
                        variable_color,
                        func_color,
                        section_color,
                    )?;

                    write!(stream, ",")?;

                    KOFileDebug::write_symbol(
                        stream,
                        sym2,
                        symstrtab,
                        symdata,
                        regular_color,
                        variable_color,
                        func_color,
                        section_color,
                    )?;
                }
            }

            writeln!(stream, "")?;
        }

        Ok(())
    }

    fn symbol_from_operand(&self, op: usize) -> Result<&KOSymbol, Box<dyn Error>> {
        self.get_symtab()?
            .get(op)
            .ok_or(format!("Tried to find symbol with index {}, but found none.", op).into())
    }

    fn write_symbol(
        stream: &mut StandardStream,
        symbol: &KOSymbol,
        symstrtab: &StringTable,
        symdata: &DataSection,
        regular_color: &ColorSpec,
        variable_color: &ColorSpec,
        func_color: &ColorSpec,
        section_color: &ColorSpec,
    ) -> DumpResult {
        let sym_name_idx = symbol.name_idx();
        let sym_name = symstrtab
            .get(sym_name_idx)
            .ok_or("Error getting name of symbol, index invalid")?;

        match symbol.sym_type() {
            kerbalobjects::kofile::symbols::SymType::Func => {
                stream.set_color(func_color)?;
                write!(stream, "{}", sym_name)?;
            }
            kerbalobjects::kofile::symbols::SymType::Section => {
                stream.set_color(section_color)?;
                write!(stream, "{}", sym_name)?;
            }
            kerbalobjects::kofile::symbols::SymType::File => {
                write!(stream, "{}", sym_name)?;
            }
            kerbalobjects::kofile::symbols::SymType::Object => {
                stream.set_color(variable_color)?;
                write!(stream, "{}", sym_name)?;
            }
            kerbalobjects::kofile::symbols::SymType::NoType => {
                let value = symdata
                    .get(symbol.value_idx())
                    .ok_or("Value referenced by symbol does not exist")?;

                super::write_kosvalue(stream, value, regular_color, variable_color)?;
            }
            kerbalobjects::kofile::symbols::SymType::Unknown => {
                write!(stream, "UNKNOWN")?;
            }
        }

        Ok(())
    }

    fn dump_symbols(
        &self,
        stream: &mut StandardStream,
        regular_color: &ColorSpec,
        name_color: &ColorSpec,
        value_color: &ColorSpec,
        size_color: &ColorSpec,
        bind_color: &ColorSpec,
        type_color: &ColorSpec,
        index_color: &ColorSpec,
    ) -> DumpResult {
        stream.set_color(regular_color)?;
        writeln!(stream, "\nSymbol Tables:")?;

        let symstrtab = self.get_symstrtab()?;

        for symbol_table in self.kofile.sym_tabs() {
            let sh_index = symbol_table.section_index();

            let name = self.get_section_name(sh_index)?;

            writeln!(stream, "Table {}", name)?;

            writeln!(
                stream,
                "{:<16}{:<10}{:<8}{:<10}{:<10}{}",
                "Name", "Value", "Size", "Binding", "Type", "Section"
            )?;

            for symbol in symbol_table.symbols() {
                let symbol_name = symstrtab.get(symbol.name_idx());

                match symbol_name {
                    Some(symbol_name) => {
                        stream.set_color(name_color)?;
                        write!(stream, "{:<16.16}", symbol_name)?;
                    }
                    None => {
                        write!(stream, "{:<16}", "")?;
                    }
                }

                stream.set_color(value_color)?;
                write!(stream, "{:0>8x}  ", symbol.value_idx())?;

                stream.set_color(size_color)?;
                write!(stream, "{:0>4x}    ", symbol.size())?;

                let bind_str = match symbol.sym_bind() {
                    kerbalobjects::kofile::symbols::SymBind::Local => "LOCAL",
                    kerbalobjects::kofile::symbols::SymBind::Global => "GLOBAL",
                    kerbalobjects::kofile::symbols::SymBind::Extern => "EXTERN",
                    kerbalobjects::kofile::symbols::SymBind::Unknown => "UNKNOWN",
                };

                stream.set_color(bind_color)?;
                write!(stream, "{:<10}", bind_str)?;

                let kind_str = match symbol.sym_type() {
                    kerbalobjects::kofile::symbols::SymType::Func => "FUNC",
                    kerbalobjects::kofile::symbols::SymType::File => "FILE",
                    kerbalobjects::kofile::symbols::SymType::NoType => "NOTYPE",
                    kerbalobjects::kofile::symbols::SymType::Object => "OBJECT",
                    kerbalobjects::kofile::symbols::SymType::Section => "SECTION",
                    kerbalobjects::kofile::symbols::SymType::Unknown => "UNKNOWN",
                };

                stream.set_color(type_color)?;
                write!(stream, "{:<10}", kind_str)?;

                stream.set_color(index_color)?;
                writeln!(stream, "{}", symbol.sh_idx())?;
            }
        }

        Ok(())
    }

    fn dump_data(
        &self,
        stream: &mut StandardStream,
        regular_color: &ColorSpec,
        type_color: &ColorSpec,
        variable_color: &ColorSpec,
    ) -> DumpResult {
        stream.set_color(regular_color)?;
        writeln!(stream, "\nSymbol Data Sections:")?;

        for data_section in self.kofile.data_sections() {
            let sh_index = data_section.section_index();

            let name = self.get_section_name(sh_index)?;

            writeln!(stream, "Section {}", name)?;
            writeln!(stream, "{:<12}{:<12}{}", "Index", "Type", "Value")?;

            for (i, value) in data_section.data().enumerate() {
                write!(stream, "  {:<10}", i)?;

                stream.set_color(type_color)?;
                match value {
                    kerbalobjects::KOSValue::Null => {
                        writeln!(stream, "NULL")?;
                    }
                    kerbalobjects::KOSValue::Bool(b) => {
                        write!(stream, "{:<12}", "BOOL")?;
                        stream.set_color(regular_color)?;
                        write!(stream, "{}", if *b { "true" } else { "false" })?;
                    }
                    kerbalobjects::KOSValue::Byte(b) => {
                        write!(stream, "{:<12}", "BYTE")?;
                        stream.set_color(regular_color)?;
                        write!(stream, "{}", b)?;
                    }
                    kerbalobjects::KOSValue::Int16(i) => {
                        write!(stream, "{:<12}", "INT16")?;
                        stream.set_color(regular_color)?;
                        write!(stream, "{}", i)?;
                    }
                    kerbalobjects::KOSValue::Int32(i) => {
                        write!(stream, "{:<12}", "INT32")?;
                        stream.set_color(regular_color)?;
                        write!(stream, "{}", i)?;
                    }
                    kerbalobjects::KOSValue::Float(f) => {
                        write!(stream, "{:<12}", "FLOAT")?;
                        stream.set_color(regular_color)?;
                        write!(stream, "{:.5}", f)?;
                    }
                    kerbalobjects::KOSValue::Double(d) => {
                        write!(stream, "{:<12}", "DOUBLE")?;
                        stream.set_color(regular_color)?;
                        write!(stream, "{:.5}", d)?;
                    }
                    kerbalobjects::KOSValue::String(s) => {
                        write!(stream, "{:<12}", "STRING")?;
                        stream.set_color(regular_color)?;
                        write!(stream, "\"")?;
                        if s.starts_with("$") {
                            stream.set_color(variable_color)?;
                        } else {
                            stream.set_color(regular_color)?;
                        }
                        write!(stream, "{}", s)?;
                        stream.set_color(regular_color)?;
                        write!(stream, "\"")?;
                    }
                    kerbalobjects::KOSValue::ArgMarker => {
                        write!(stream, "{:<12}", "ARGMARKER")?;
                    }
                    kerbalobjects::KOSValue::ScalarInt(i) => {
                        write!(stream, "{:<12}", "SCALARINT")?;
                        stream.set_color(regular_color)?;
                        write!(stream, "{}", i)?;
                    }
                    kerbalobjects::KOSValue::ScalarDouble(d) => {
                        write!(stream, "{:<12}", "SCALARDOUBLE")?;
                        stream.set_color(regular_color)?;
                        write!(stream, "{}", d)?;
                    }
                    kerbalobjects::KOSValue::BoolValue(b) => {
                        write!(stream, "{:<12}", "SCALARDOUBLE")?;
                        stream.set_color(regular_color)?;
                        write!(stream, "{}", if *b { "true" } else { "false" })?;
                    }
                    kerbalobjects::KOSValue::StringValue(s) => {
                        write!(stream, "{:<12}", "STRINGVALUE")?;
                        if s.starts_with("$") {
                            stream.set_color(variable_color)?;
                        } else {
                            stream.set_color(regular_color)?;
                        }
                        write!(stream, "\"{}\"", s)?;
                    }
                }
                writeln!(stream, "")?;
            }
        }

        Ok(())
    }

    fn dump_section_headers(
        &self,
        stream: &mut StandardStream,
        regular_color: &ColorSpec,
        name_color: &ColorSpec,
        type_color: &ColorSpec,
        size_color: &ColorSpec,
    ) -> DumpResult {
        stream.set_color(regular_color)?;
        writeln!(stream, "\nSections:")?;

        writeln!(
            stream,
            "{:<7}{:<16}{:<12}{:<12}",
            "Index", "Name", "Kind", "Size"
        )?;

        for (i, header) in self.kofile.section_headers().enumerate() {
            write!(stream, "{:<7}", i)?;
            stream.set_color(name_color)?;
            let name = self.get_section_name(i)?;
            write!(stream, "{:<16}", name)?;
            stream.set_color(type_color)?;
            write!(stream, "{:<12}", KOFileDebug::kind_as_str(header.kind()))?;
            stream.set_color(size_color)?;
            writeln!(stream, "{:<12}\n", header.size())?;
            stream.set_color(regular_color)?;
        }

        Ok(())
    }

    fn kind_as_str(kind: SectionKind) -> &'static str {
        match kind {
            SectionKind::Null => "NULL",
            SectionKind::Rel => "REL",
            SectionKind::Data => "DATA",
            SectionKind::SymTab => "SYMTAB",
            SectionKind::StrTab => "STRTAB",
            SectionKind::Debug => "DEBUG",
            SectionKind::Unknown => "UNKNOWN",
        }
    }

    fn dump_info(&self, stream: &mut StandardStream) -> DumpResult {
        writeln!(stream, "\nKO File Info:")?;

        if let Some(comment_section) =
            self.kofile
                .str_tabs()
                .find(|x| match self.get_section_name(x.section_index()) {
                    Ok(name) => name == ".comment",
                    Err(_) => false,
                })
        {
            match comment_section.get(1) {
                Some(comment) => {
                    writeln!(stream, "  {}", comment)?;
                }
                None => {
                    writeln!(stream, "  Comment section empty.")?;
                }
            }
        } else {
            writeln!(stream, "  No info")?;
        }

        Ok(())
    }

    fn dump_strtabs(
        &self,
        stream: &mut StandardStream,
        regular_color: &ColorSpec,
        index_color: &ColorSpec,
        str_color: &ColorSpec,
    ) -> DumpResult {
        stream.set_color(regular_color)?;
        writeln!(stream, "\nString tables:")?;

        for strtab in self.kofile.str_tabs() {
            let sh_index = strtab.section_index();

            let name = self.get_section_name(sh_index)?;

            writeln!(stream, "{}", name)?;

            let mut index = 1;

            for s in strtab.strings() {
                write!(stream, "  [")?;

                stream.set_color(index_color)?;

                write!(stream, "{:5}", index)?;

                stream.set_color(regular_color)?;

                write!(stream, "]  ")?;

                stream.set_color(str_color)?;

                writeln!(stream, "{}", s)?;

                stream.set_color(regular_color)?;

                index += s.len() + 1;
            }
        }

        Ok(())
    }

    fn dump_ko_header(&self, stream: &mut StandardStream) -> DumpResult {
        writeln!(stream, "\nFile header:")?;

        writeln!(stream, "\tVersion: {}", self.kofile.version())?;

        writeln!(stream, "\tShstrtab Index: {}", self.kofile.strtab_index())?;

        writeln!(
            stream,
            "\tNumber of sections: {}",
            self.kofile.section_count()
        )?;

        Ok(())
    }
}
