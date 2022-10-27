use kerbalobjects::ko::sections::{
    DataIdx, DataSection, FuncSection, InstrIdx, SectionKind, StringIdx, StringTable, SymbolIdx,
    SymbolTable,
};
use kerbalobjects::ko::symbols::OperandIndex;
use kerbalobjects::ko::{KOFile, SectionIdx};
use kerbalobjects::KOSValue;
use std::error::Error;
use std::io::Write;
use termcolor::ColorSpec;
use termcolor::StandardStream;
use termcolor::WriteColor;

use crate::output::DynResult;
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

        if config.reloc || config.full_contents {
            self.dump_relocs(stream, &no_color, &purple)?;
        }

        if config.disassemble || config.full_contents {
            self.dump_func_sections(
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

        if let Some(disassemble_symbol) = &config.disassemble_symbol {
            self.dump_func_by_symbol(
                stream,
                disassemble_symbol,
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

    fn get_section_name(&self, sh_index: SectionIdx) -> Result<&str, Box<dyn Error>> {
        let header = self.kofile.get_section_header(sh_index).ok_or(format!(
            "Failed to find KO file section header for string table with index {}",
            u16::from(sh_index)
        ))?;

        let name = self.kofile.get_header_name(header).ok_or(format!(
            "Failed to find the string table with section {}'s name in KO file",
            u16::from(sh_index)
        ))?;

        Ok(name)
    }

    fn dump_relocs(
        &self,
        stream: &mut StandardStream,
        regular_color: &ColorSpec,
        index_color: &ColorSpec,
    ) -> DumpResult {
        stream.set_color(regular_color)?;

        writeln!(stream, "\nRelocation data sections:")?;

        if self.kofile.reld_sections().len() != 0 {
            for reld_section in self.kofile.reld_sections() {
                let name = self.get_section_name(reld_section.section_index())?;

                writeln!(stream, "Reld section {}:", name)?;

                writeln!(
                    stream,
                    "{:<12}{:<14}{:<12}{:<12}",
                    "Section", "Instruction", "Operand", "Symbol index"
                )?;

                stream.set_color(index_color)?;

                for reld_entry in reld_section.entries() {
                    writeln!(
                        stream,
                        "{:<12}{:0>8}      {:<12}{:0>8}",
                        u16::from(reld_entry.section_index),
                        u32::from(reld_entry.instr_index),
                        u8::from(reld_entry.operand_index),
                        u32::from(reld_entry.symbol_index)
                    )?;
                }
            }
        } else {
            writeln!(stream, "None.")?;
        }

        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    fn dump_func_by_symbol(
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
        let mut func_section_found = None;

        let data_section = self
            .kofile
            .data_section_by_name(".data")
            .ok_or(".data section not found")?;

        let symtab_opt = self.kofile.sym_tab_by_name(".symtab");
        let symstrtab_opt = self.kofile.str_tab_by_name(".symstrtab");

        for func_section in self.kofile.func_sections() {
            let sh_index = func_section.section_index();
            let name = self.get_section_name(sh_index)?;

            if name == symbol_text {
                func_section_found = Some(func_section);
                break;
            }

            // Loop through each instruction in the section
            for (i, instr) in func_section.instructions().enumerate() {
                let relocs = self.get_relocated(sh_index, InstrIdx::from(i));

                match instr {
                    kerbalobjects::ko::Instr::ZeroOp(_) => {}
                    kerbalobjects::ko::Instr::OneOp(_, op1) => {
                        if self.operand_matches_str(
                            *op1,
                            relocs.0,
                            data_section,
                            symtab_opt,
                            symstrtab_opt,
                            symbol_text,
                        )? {
                            func_section_found = Some(func_section);
                            break;
                        }
                    }
                    kerbalobjects::ko::Instr::TwoOp(_, op1, op2) => {
                        func_section_found = if self.operand_matches_str(
                            *op1,
                            relocs.0,
                            data_section,
                            symtab_opt,
                            symstrtab_opt,
                            symbol_text,
                        )? || self.operand_matches_str(
                            *op2,
                            relocs.1,
                            data_section,
                            symtab_opt,
                            symstrtab_opt,
                            symbol_text,
                        )? {
                            Some(func_section)
                        } else {
                            None
                        };

                        if func_section_found.is_some() {
                            break;
                        }
                    }
                }
            }

            if func_section_found.is_some() {
                break;
            }
        }

        match func_section_found {
            Some(section) => {
                self.dump_func_section(
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

    fn operand_matches_str(
        &self,
        op: DataIdx,
        reloc: (bool, SymbolIdx),
        data_section: &DataSection,
        symtab_opt: Option<&SymbolTable>,
        symstrtab_opt: Option<&StringTable>,
        symbol_text: &String,
    ) -> DynResult<bool> {
        if reloc.0 {
            let symtab =
                symtab_opt.ok_or("Instruction points to symbol, but symbol table not found")?;
            let symstrtab = symstrtab_opt
                .ok_or("Instruction points to symbol, but symbol string table not found")?;

            let sym = symtab.get(reloc.1).ok_or(format!(
                "Reld entry symbol index invalid: {}",
                u32::from(reloc.1)
            ))?;

            let sym_name = symstrtab.get(sym.name_idx).ok_or(format!(
                "Symbol name index invalid: {}",
                u32::from(sym.name_idx)
            ))?;

            if sym_name == symbol_text {
                return Ok(true);
            }
        } else {
            let value = data_section
                .get(op)
                .ok_or(format!("Instruction data index invalid: {}", u32::from(op)))?;

            match value {
                KOSValue::String(s) | KOSValue::StringValue(s) => {
                    if s == symbol_text {
                        return Ok(true);
                    }
                }
                _ => {}
            }
        }

        Ok(false)
    }

    #[allow(clippy::too_many_arguments)]
    fn dump_func_sections(
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
        stream.set_color(regular_color)?;

        writeln!(stream, "\nFunction sections: ")?;

        for func_section in self.kofile.func_sections() {
            self.dump_func_section(
                stream,
                regular_color,
                index_color,
                mnemonic_color,
                variable_color,
                func_color,
                section_color,
                show_labels,
                show_raw_instr,
                func_section,
            )?;
        }

        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    fn dump_func_section(
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
        func_section: &FuncSection,
    ) -> DumpResult {
        stream.set_color(regular_color)?;

        let sh_index = func_section.section_index();

        let name = self.get_section_name(sh_index)?;

        let data_section = self
            .kofile
            .data_section_by_name(".data")
            .ok_or("Could not find KO file .data section")?;

        let symtab_opt = self.kofile.sym_tab_by_name(".symtab");
        let symstrtab_opt = self.kofile.str_tab_by_name(".symstrtab");

        writeln!(stream, "{}:", name)?;

        for (i, instr) in func_section.instructions().enumerate() {
            write!(stream, "  ")?;

            if show_labels {
                stream.set_color(index_color)?;
                write!(stream, "{:0>8x} ", i + 1)?;
                stream.set_color(regular_color)?;
            }

            let instr_opcode = if show_raw_instr {
                match instr {
                    kerbalobjects::ko::Instr::ZeroOp(opcode) => {
                        write!(stream, "{:0>2x} {:<8} {:<8} ", u8::from(*opcode), "", "")?;
                        *opcode
                    }
                    kerbalobjects::ko::Instr::OneOp(opcode, op1) => {
                        write!(
                            stream,
                            "{:0>2x} {:0>8x} {:<8} ",
                            u8::from(*opcode),
                            u32::from(*op1),
                            ""
                        )?;
                        *opcode
                    }
                    kerbalobjects::ko::Instr::TwoOp(opcode, op1, op2) => {
                        write!(
                            stream,
                            "{:0>2x} {:0>8x} {:0>8x} ",
                            u8::from(*opcode),
                            u32::from(*op1),
                            u32::from(*op2)
                        )?;
                        *opcode
                    }
                }
            } else {
                instr.opcode()
            };

            let instr_mnemonic: &str = instr_opcode.into();

            stream.set_color(mnemonic_color)?;
            write!(stream, " {:<5}", instr_mnemonic)?;
            stream.set_color(regular_color)?;

            let relocs = self.get_relocated(sh_index, InstrIdx::from(i));

            match instr {
                kerbalobjects::ko::Instr::ZeroOp(_) => {}
                kerbalobjects::ko::Instr::OneOp(_, op1) => {
                    // If this operand has a relocation entry
                    if relocs.0 .0 {
                        let symtab = symtab_opt
                            .ok_or("Instruction requires symbol, but symbol table not found")?;
                        let symstrtab = symstrtab_opt.ok_or(
                            "Instruction requires symbol, but symbol string table not found",
                        )?;

                        let sym1 = symtab.get(relocs.0 .1).ok_or(format!(
                            "Reld entry symbol index invalid: {}",
                            u32::from(relocs.0 .1)
                        ))?;

                        let sym1_name = symstrtab.get(sym1.name_idx).ok_or(format!(
                            "Symbol has invalid name index: {}",
                            u32::from(sym1.name_idx)
                        ))?;

                        match sym1.sym_type {
                            kerbalobjects::ko::symbols::SymType::Func => {
                                stream.set_color(func_color)?;
                                write!(stream, "<{}>", sym1_name)?;
                                stream.set_color(regular_color)?;
                            }
                            kerbalobjects::ko::symbols::SymType::Section => {
                                stream.set_color(section_color)?;
                                write!(stream, "<{}>", sym1_name)?;
                                stream.set_color(regular_color)?;
                            }
                            kerbalobjects::ko::symbols::SymType::NoType => {
                                stream.set_color(variable_color)?;
                                write!(stream, "<{}>", sym1_name)?;
                                stream.set_color(regular_color)?;
                            }
                            _ => {}
                        }
                    } else {
                        // This instruction has a regular value
                        let value = data_section.get(*op1).ok_or(format!(
                            "Instruction data index invalid: {}",
                            u32::from(*op1)
                        ))?;

                        super::write_kosvalue(stream, value, regular_color, variable_color)?;
                    }
                }
                kerbalobjects::ko::Instr::TwoOp(_, op1, op2) => {
                    // If this operand has a relocation entry
                    if relocs.0 .0 {
                        let symtab = symtab_opt
                            .ok_or("Instruction requires symbol, but symbol table not found")?;
                        let symstrtab = symstrtab_opt.ok_or(
                            "Instruction requires symbol, but symbol string table not found",
                        )?;

                        let sym1 = symtab.get(relocs.0 .1).ok_or(format!(
                            "Reld entry symbol index invalid: {}",
                            u32::from(relocs.0 .1)
                        ))?;

                        let sym1_name = symstrtab.get(sym1.name_idx).ok_or(format!(
                            "Symbol has invalid name index: {}",
                            u32::from(sym1.name_idx)
                        ))?;

                        match sym1.sym_type {
                            kerbalobjects::ko::symbols::SymType::Func => {
                                stream.set_color(func_color)?;
                                write!(stream, "<{}>", sym1_name)?;
                                stream.set_color(regular_color)?;
                            }
                            kerbalobjects::ko::symbols::SymType::Section => {
                                stream.set_color(section_color)?;
                                write!(stream, "<{}>", sym1_name)?;
                                stream.set_color(regular_color)?;
                            }
                            kerbalobjects::ko::symbols::SymType::NoType => {
                                stream.set_color(variable_color)?;
                                write!(stream, "<{}>", sym1_name)?;
                                stream.set_color(regular_color)?;
                            }
                            _ => {}
                        }
                    } else {
                        // This instruction has a regular value
                        let value = data_section.get(*op1).ok_or(format!(
                            "Instruction data index invalid: {}",
                            u32::from(*op1)
                        ))?;

                        super::write_kosvalue(stream, value, regular_color, variable_color)?;
                    }

                    write!(stream, ", ")?;

                    // If this operand has a relocation entry
                    if relocs.1 .0 {
                        let symtab = symtab_opt
                            .ok_or("Instruction requires symbol, but symbol table not found")?;
                        let symstrtab = symstrtab_opt.ok_or(
                            "Instruction requires symbol, but symbol string table not found",
                        )?;

                        let sym2 = symtab.get(relocs.1 .1).ok_or(format!(
                            "Reld entry symbol index invalid: {}",
                            u32::from(relocs.1 .1)
                        ))?;

                        let sym2_name = symstrtab.get(sym2.name_idx).ok_or(format!(
                            "Symbol has invalid name index: {}",
                            u32::from(sym2.name_idx)
                        ))?;

                        match sym2.sym_type {
                            kerbalobjects::ko::symbols::SymType::Func => {
                                stream.set_color(func_color)?;
                                write!(stream, "<{}>", sym2_name)?;
                                stream.set_color(regular_color)?;
                            }
                            kerbalobjects::ko::symbols::SymType::Section => {
                                stream.set_color(section_color)?;
                                write!(stream, "<{}>", sym2_name)?;
                                stream.set_color(regular_color)?;
                            }
                            kerbalobjects::ko::symbols::SymType::NoType => {
                                stream.set_color(variable_color)?;
                                write!(stream, "<{}>", sym2_name)?;
                                stream.set_color(regular_color)?;
                            }
                            _ => {}
                        }
                    } else {
                        // This instruction has a regular value
                        let value = data_section.get(*op2).ok_or(format!(
                            "Instruction data index invalid: {}",
                            u32::from(*op1)
                        ))?;

                        super::write_kosvalue(stream, value, regular_color, variable_color)?;
                    }
                }
            }

            writeln!(stream)?;
        }

        Ok(())
    }

    fn get_relocated(
        &self,
        section_index: SectionIdx,
        instr_index: InstrIdx,
    ) -> ((bool, SymbolIdx), (bool, SymbolIdx)) {
        let mut first_reloc = (false, SymbolIdx::from(0u32));
        let mut second_reloc = (false, SymbolIdx::from(0u32));

        let reld_section = match self.kofile.reld_section_by_name(".reld") {
            Some(section) => section,
            None => {
                return (
                    (false, SymbolIdx::from(0u32)),
                    (false, SymbolIdx::from(0u32)),
                );
            }
        };

        for reld_entry in reld_section.entries() {
            if reld_entry.section_index == section_index && reld_entry.instr_index == instr_index {
                match reld_entry.operand_index {
                    OperandIndex::One => {
                        first_reloc = (true, reld_entry.symbol_index);
                    }
                    OperandIndex::Two => {
                        second_reloc = (true, reld_entry.symbol_index);
                    }
                }
            }
        }

        (first_reloc, second_reloc)
    }

    #[allow(clippy::too_many_arguments)]
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

        let symstrtab_opt = self.kofile.str_tab_by_name(".symstrtab");

        match symstrtab_opt {
            Some(symstrtab) => {
                for symbol_table in self.kofile.sym_tabs() {
                    let sh_index = symbol_table.section_index();

                    let name = self.get_section_name(sh_index)?;

                    writeln!(stream, "Table {}", name)?;

                    writeln!(
                        stream,
                        "{:<16}{:<10}{:<8}{:<10}{:<10}Section",
                        "Name", "Value", "Size", "Binding", "Type"
                    )?;

                    for symbol in symbol_table.symbols() {
                        let symbol_name = symstrtab.get(symbol.name_idx);

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
                        write!(stream, "{:0>8x}  ", u32::from(symbol.value_idx))?;

                        stream.set_color(size_color)?;
                        write!(stream, "{:0>4x}    ", symbol.size)?;

                        let bind_str = match symbol.sym_bind {
                            kerbalobjects::ko::symbols::SymBind::Local => "LOCAL",
                            kerbalobjects::ko::symbols::SymBind::Global => "GLOBAL",
                            kerbalobjects::ko::symbols::SymBind::Extern => "EXTERN",
                        };

                        stream.set_color(bind_color)?;
                        write!(stream, "{:<10}", bind_str)?;

                        let kind_str = match symbol.sym_type {
                            kerbalobjects::ko::symbols::SymType::Func => "FUNC",
                            kerbalobjects::ko::symbols::SymType::File => "FILE",
                            kerbalobjects::ko::symbols::SymType::NoType => "NOTYPE",
                            kerbalobjects::ko::symbols::SymType::Object => "OBJECT",
                            kerbalobjects::ko::symbols::SymType::Section => "SECTION",
                        };

                        stream.set_color(type_color)?;
                        write!(stream, "{:<10}", kind_str)?;

                        stream.set_color(index_color)?;
                        writeln!(stream, "{}", u16::from(symbol.sh_idx))?;
                    }
                }
            }
            None => {
                writeln!(stream, "None.")?;
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
            writeln!(stream, "{:<12}{:<12}Value", "Index", "Type")?;

            for (i, value) in data_section.data().enumerate() {
                write!(stream, "  {:<10}", i)?;

                stream.set_color(type_color)?;
                match value {
                    kerbalobjects::KOSValue::Null => {
                        write!(stream, "NULL")?;
                        stream.set_color(regular_color)?;
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
                        if s.starts_with('$') {
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
                        stream.set_color(regular_color)?;
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
                        if s.starts_with('$') {
                            stream.set_color(variable_color)?;
                        } else {
                            stream.set_color(regular_color)?;
                        }
                        write!(stream, "\"{}\"", s)?;
                    }
                }
                writeln!(stream)?;
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
            let name = self.get_section_name(SectionIdx::from(i as u16))?;
            write!(stream, "{:<16}", name)?;
            stream.set_color(type_color)?;
            write!(
                stream,
                "{:<12}",
                KOFileDebug::kind_as_str(header.section_kind)
            )?;
            stream.set_color(size_color)?;
            writeln!(stream, "{:<12}\n", header.size)?;
            stream.set_color(regular_color)?;
        }

        Ok(())
    }

    fn kind_as_str(kind: SectionKind) -> &'static str {
        match kind {
            SectionKind::Null => "NULL",
            SectionKind::Reld => "RELD",
            SectionKind::Func => "FUNC",
            SectionKind::Data => "DATA",
            SectionKind::SymTab => "SYMTAB",
            SectionKind::StrTab => "STRTAB",
            SectionKind::Debug => "DEBUG",
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
            match comment_section.get(StringIdx::from(1u32)) {
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

            for s in strtab.strings().skip(1) {
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

        writeln!(stream, "\tVersion: {}", self.kofile.header().version)?;

        writeln!(
            stream,
            "\tShstrtab Index: {}",
            u16::from(self.kofile.header().shstrtab_idx)
        )?;

        writeln!(
            stream,
            "\tNumber of section headers: {}",
            self.kofile.header().num_headers
        )?;

        Ok(())
    }
}
