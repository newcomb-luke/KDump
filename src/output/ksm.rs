use crate::{CLIConfig, GREEN, LIGHT_RED, NO_COLOR};
use crate::{DARK_RED, ORANGE, PURPLE};
use kerbalobjects::ksm::sections::DebugEntry;
use kerbalobjects::ksm::sections::DebugRange;
use kerbalobjects::ksm::sections::{ArgIndex, CodeSection};
use kerbalobjects::ksm::Instr;
use kerbalobjects::ksm::KSMFile;
use kerbalobjects::KOSValue;
use kerbalobjects::Opcode;
use std::io::Write;
use termcolor::StandardStream;
use termcolor::WriteColor;

use super::{DumpResult, DynResult};

pub struct KSMFileDebug {
    ksmfile: KSMFile,
}

impl KSMFileDebug {
    pub fn new(ksmfile: KSMFile) -> Self {
        KSMFileDebug { ksmfile }
    }

    pub fn dump(&self, stream: &mut StandardStream, config: &CLIConfig) -> DumpResult {
        if config.info {
            writeln!(stream, "\nKSM File Info:")?;
            writeln!(stream, "\t{}", self.get_info())?;
        }

        if config.argument_section || config.full_contents {
            self.dump_argument_section(stream)?;
        }

        if config.disassemble || config.full_contents {
            self.dump_code_sections(stream, config)?;
        }

        if let Some(disassemble_symbol) = &config.disassemble_symbol {
            self.dump_code_by_symbol(stream, config, disassemble_symbol)?;
        }

        if config.full_contents {
            self.dump_debug(stream)?;
        }

        Ok(())
    }

    fn get_info(&self) -> String {
        let value = self.ksmfile.arg_section.arguments().next();

        get_info(value)
    }

    fn dump_debug(&self, stream: &mut StandardStream) -> DumpResult {
        stream.set_color(&NO_COLOR)?;

        writeln!(stream, "\nDebug section:")?;

        let max_line_number = self.max_debug_line_number();
        let max_width = max_line_number.to_string().len();

        for debug_entry in self.ksmfile.debug_section.debug_entries() {
            write!(
                stream,
                "  Line {:>width$}, ",
                debug_entry.line_number,
                width = max_width
            )?;

            let num_ranges = debug_entry.number_ranges();

            match num_ranges {
                1 => {
                    write!(stream, "1 range: ")?;
                }
                _ => {
                    write!(stream, "{} ranges: ", num_ranges)?;
                }
            }

            for (index, range) in debug_entry.ranges().enumerate() {
                write!(stream, "[{:0>6x}, {:0>6x}]", range.start, range.end)?;

                if index < num_ranges - 1 {
                    write!(stream, ",")?;
                }
            }

            writeln!(stream)?;
        }

        Ok(())
    }

    fn dump_code_by_symbol(
        &self,
        stream: &mut StandardStream,
        config: &CLIConfig,
        symbol: &String,
    ) -> DumpResult {
        let mut index = 1;
        let mut addr = 0;
        let mut found_section = None;

        for code_section in self.ksmfile.code_sections() {
            let matches = match code_section.section_type {
                kerbalobjects::ksm::sections::CodeType::Main => symbol.eq_ignore_ascii_case("main"),
                kerbalobjects::ksm::sections::CodeType::Initialization => {
                    symbol.eq_ignore_ascii_case("init")
                }
                kerbalobjects::ksm::sections::CodeType::Function => false,
            };

            if matches {
                found_section = Some(code_section);
                break;
            } else {
                for (in_func_index, instr) in code_section.instructions().enumerate() {
                    let matches = match instr {
                        Instr::ZeroOp(_) => false,
                        Instr::OneOp(_, op1) => {
                            let val1 = self.value_from_operand(*op1).ok_or(format!(
                                "Instruction number {} references invalid argument index: {:x}",
                                in_func_index,
                                usize::from(*op1)
                            ))?;

                            match val1 {
                                KOSValue::String(s) | KOSValue::StringValue(s) => s == symbol,
                                _ => false,
                            }
                        }
                        Instr::TwoOp(_, op1, op2) => {
                            let val1 = self.value_from_operand(*op1).ok_or(format!(
                                "Instruction number {} references invalid argument index: {:x}",
                                in_func_index,
                                usize::from(*op1)
                            ))?;
                            let val2 = self.value_from_operand(*op2).ok_or(format!(
                                "Instruction number {} references invalid argument index: {:x}",
                                in_func_index,
                                usize::from(*op2)
                            ))?;

                            let match1 = match val1 {
                                KOSValue::String(s) | KOSValue::StringValue(s) => s == symbol,
                                _ => false,
                            };
                            let match2 = match val2 {
                                KOSValue::String(s) | KOSValue::StringValue(s) => s == symbol,
                                _ => false,
                            };

                            match1 || match2
                        }
                    };

                    if matches {
                        found_section = Some(code_section);
                        break;
                    }
                }
            }

            index += code_section.instructions().len() as i32;

            addr += 2; // Offsets for the header bytes
            for instr in code_section.instructions() {
                addr += self.instr_size(instr);
            }
        }

        match found_section {
            Some(code_section) => {
                self.dump_code_section(
                    stream,
                    code_section,
                    index,
                    addr,
                    config.line_numbers,
                    !config.show_no_labels,
                    !config.show_no_raw_instr,
                )?;
            }
            None => {
                writeln!(stream, "\nNo section found with that symbol.")?;
            }
        }

        Ok(())
    }

    fn dump_code_sections(&self, stream: &mut StandardStream, config: &CLIConfig) -> DumpResult {
        let mut index = 1;
        let mut addr = 0;

        for code_section in self.ksmfile.code_sections() {
            if code_section.instructions().len() != 0 {
                let (new_index, new_addr) = self.dump_code_section(
                    stream,
                    code_section,
                    index,
                    addr,
                    config.line_numbers,
                    !config.show_no_labels,
                    !config.show_no_raw_instr,
                )?;

                index = new_index;
                addr = new_addr;
            } else {
                index += code_section.instructions().len() as i32;

                addr += 2; // Offsets for the header bytes
                for instr in code_section.instructions() {
                    addr += self.instr_size(instr);
                }
            }
        }

        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    fn dump_code_section(
        &self,
        stream: &mut StandardStream,
        code_section: &CodeSection,
        start_index: i32,
        start_addr: usize,
        show_line_numbers: bool,
        show_labels: bool,
        show_raw_instr: bool,
    ) -> DynResult<(i32, usize)> {
        let section_type = code_section.section_type;
        let addr_width = self.ksmfile.arg_section.num_index_bytes() as u8 as usize;

        let name = match section_type {
            kerbalobjects::ksm::sections::CodeType::Main => "MAIN",
            kerbalobjects::ksm::sections::CodeType::Initialization => "INIT",
            kerbalobjects::ksm::sections::CodeType::Function => {
                match code_section.instructions().next() {
                    Some(&Instr::OneOp(opcode, op1)) => {
                        if opcode == Opcode::Lbrt {
                            let operand = self.value_from_operand(op1).ok_or(format!(
                                "Instruction number {} references invalid argument index: {:x}",
                                0,
                                usize::from(op1)
                            ))?;

                            match operand {
                                KOSValue::String(s) | KOSValue::StringValue(s) => {
                                    // If this is a kOS-compiled function
                                    if s.contains('`') {
                                        s.split('`').next().unwrap()
                                    } else {
                                        s
                                    }
                                }
                                _ => "FUNC",
                            }
                        } else {
                            "FUNC"
                        }
                    }
                    _ => "FUNC",
                }
            }
        };

        stream.set_color(&NO_COLOR)?;
        writeln!(stream, "\n{}:", name)?;

        let mut label = String::from("@000001");
        let mut index = start_index;
        let mut addr = start_addr + 2;

        let max_line_number = self.max_debug_line_number();
        let max_width = max_line_number.to_string().len();

        for (in_func_index, instr) in code_section.instructions().enumerate() {
            let instr_size = self.instr_size(instr);

            if show_line_numbers {
                let debug_entry = self.find_entry_with_addr(addr);

                match debug_entry {
                    Some((entry, range)) => {
                        let line_num = entry.line_number;
                        let range_start = range.start;
                        let range_end = range.end;
                        let range_middle = ((range_end - range_start) / 2) + range_start;
                        let operand_length = instr_size - 1;

                        let state = if addr == range_start
                            && range_start + operand_length == range_end
                        {
                            3
                        } else if addr == range_start {
                            let next_instr_option =
                                code_section.instructions().nth(index as usize + 1);

                            match next_instr_option {
                                Some(next_instr) => {
                                    if addr + operand_length + self.instr_size(next_instr)
                                        == range_end
                                    {
                                        5
                                    } else {
                                        0
                                    }
                                }
                                None => 0,
                            }
                        } else if addr + operand_length == range_end {
                            4
                        } else if range_middle >= addr && (range_middle <= (addr + operand_length))
                        {
                            2
                        } else if addr + operand_length < range_end && addr > range_start {
                            1
                        } else {
                            6
                        };

                        let num_str = match state {
                            2 | 3 | 5 => line_num.to_string(),
                            _ => String::new(),
                        };

                        let art = match state {
                            0 => " ╔═",
                            1 => " ║ ",
                            2 => "═╣ ",
                            3 => "═══",
                            4 => " ╚═",
                            5 => "═╦═",
                            _ => "   ",
                        };

                        stream.set_color(&ORANGE)?;
                        write!(stream, "   {:>width$} {} ", num_str, art, width = max_width)?;
                        stream.set_color(&NO_COLOR)?;
                    }
                    None => {
                        write!(stream, "   {:>width$}     ", "", width = max_width)?;
                    }
                }
            } else {
                write!(stream, "  ")?;
            }

            let instr_opcode = match instr {
                Instr::ZeroOp(opcode) => *opcode,
                Instr::OneOp(opcode, _) => *opcode,
                Instr::TwoOp(opcode, _, _) => *opcode,
            };

            let is_lbrt = instr_opcode == Opcode::Lbrt;

            if show_labels {
                stream.set_color(&PURPLE)?;

                if is_lbrt {
                    write!(stream, "{:7} ", "")?;
                } else {
                    write!(stream, "{:<7} ", label)?;
                }
            }

            stream.set_color(&NO_COLOR)?;

            if is_lbrt {
                if let &Instr::OneOp(_, op) = instr {
                    let arg = self.value_from_operand(op).ok_or(format!(
                        "Instruction number {} references invalid argument index: {:x}",
                        in_func_index,
                        usize::from(op)
                    ))?;

                    if let KOSValue::String(s) = arg {
                        label = s.clone();

                        if label.starts_with('@') {
                            // Makes @0013 @000013
                            label.insert_str(1, "00");
                        }
                    }

                    label.truncate(7);
                }
            }
            // If it isn't a label reset
            else {
                index += 1;
                label = format!("@{:>06}", index);
            }

            addr += instr_size;

            if show_raw_instr {
                match instr {
                    Instr::ZeroOp(opcode) => {
                        write!(
                            stream,
                            "{:x} {:<width$} {:<width$}",
                            u8::from(*opcode),
                            "",
                            "",
                            width = addr_width * 2
                        )?;
                    }
                    Instr::OneOp(opcode, op1) => {
                        write!(
                            stream,
                            "{:x} {:0>width$x} {:<width$}",
                            u8::from(*opcode),
                            usize::from(*op1),
                            "",
                            width = addr_width * 2
                        )?;
                    }
                    Instr::TwoOp(opcode, op1, op2) => {
                        write!(
                            stream,
                            "{:x} {:0>width$x} {:0>width$x}",
                            u8::from(*opcode),
                            usize::from(*op1),
                            usize::from(*op2),
                            width = addr_width * 2
                        )?;
                    }
                }
            }

            stream.set_color(&DARK_RED)?;

            let mnemonic: &str = instr_opcode.into();

            write!(stream, "  {:<6}", mnemonic)?;

            stream.set_color(&NO_COLOR)?;

            match instr {
                Instr::ZeroOp(_) => {}
                Instr::OneOp(_, op1) => {
                    let val1 = self.value_from_operand(*op1).ok_or(format!(
                        "Instruction number {} references invalid argument index: {:x}",
                        in_func_index,
                        usize::from(*op1)
                    ))?;

                    super::write_kosvalue(stream, val1)?;
                }
                Instr::TwoOp(_, op1, op2) => {
                    let val1 = self.value_from_operand(*op1).ok_or(format!(
                        "Instruction number {} references invalid argument index: {:x}",
                        in_func_index,
                        usize::from(*op1)
                    ))?;
                    let val2 = self.value_from_operand(*op2).ok_or(format!(
                        "Instruction number {} references invalid argument index: {:x}",
                        in_func_index,
                        usize::from(*op2)
                    ))?;

                    super::write_kosvalue(stream, val1)?;

                    write!(stream, ",")?;

                    super::write_kosvalue(stream, val2)?;
                }
            }

            writeln!(stream)?;
        }

        Ok((index, addr))
    }

    fn instr_size(&self, instr: &Instr) -> usize {
        let addr_width = self.ksmfile.arg_section.num_index_bytes() as usize;

        match instr {
            Instr::ZeroOp(_) => 1,
            Instr::OneOp(_, _) => 1 + addr_width,
            Instr::TwoOp(_, _, _) => 1 + addr_width * 2,
        }
    }

    fn max_debug_line_number(&self) -> isize {
        let mut max = 0;

        for debug_entry in self.ksmfile.debug_section.debug_entries() {
            max = max.max(debug_entry.line_number);
        }

        max
    }

    fn find_entry_with_addr(&self, addr: usize) -> Option<(&DebugEntry, &DebugRange)> {
        let debug_section = &self.ksmfile.debug_section;

        for debug_entry in debug_section.debug_entries() {
            for debug_range in debug_entry.ranges() {
                if addr >= debug_range.start && addr <= debug_range.end {
                    return Some((debug_entry, debug_range));
                }
            }
        }

        None
    }

    fn value_from_operand(&self, op: ArgIndex) -> Option<&KOSValue> {
        self.ksmfile.arg_section.get(op)
    }

    fn dump_argument_section(&self, stream: &mut StandardStream) -> DumpResult {
        let arg_section = &self.ksmfile.arg_section;
        let addr_width = arg_section.num_index_bytes() as usize;

        stream.set_color(&NO_COLOR)?;

        writeln!(stream, "\nArgument section:")?;

        writeln!(
            stream,
            "  {:18}{:<12}{:<24}",
            format!(
                "Index ({} byte{})",
                addr_width,
                if addr_width > 1 { "s" } else { "" }
            ),
            "Type",
            "Value",
        )?;

        let mut index = 3;

        for value in arg_section.arguments() {
            stream.set_color(&NO_COLOR)?;

            let index_str = format!("  {:0>width$x}", index, width = addr_width * 2);

            write!(stream, "{:<20}", index_str)?;

            index += value.size_bytes();

            stream.set_color(&GREEN)?;
            match value {
                KOSValue::Null => {
                    write!(stream, "NULL")?;
                    stream.set_color(&NO_COLOR)?;
                }
                KOSValue::Bool(b) => {
                    write!(stream, "{:<12}", "BOOL")?;
                    stream.set_color(&NO_COLOR)?;
                    write!(stream, "{}", if *b { "true" } else { "false" })?;
                }
                KOSValue::Byte(b) => {
                    write!(stream, "{:<12}", "BYTE")?;
                    stream.set_color(&NO_COLOR)?;
                    write!(stream, "{}", b)?;
                }
                KOSValue::Int16(i) => {
                    write!(stream, "{:<12}", "INT16")?;
                    stream.set_color(&NO_COLOR)?;
                    write!(stream, "{}", i)?;
                }
                KOSValue::Int32(i) => {
                    write!(stream, "{:<12}", "INT32")?;
                    stream.set_color(&NO_COLOR)?;
                    write!(stream, "{}", i)?;
                }
                KOSValue::Float(f) => {
                    write!(stream, "{:<12}", "FLOAT")?;
                    stream.set_color(&NO_COLOR)?;
                    write!(stream, "{:.5}", f)?;
                }
                KOSValue::Double(d) => {
                    write!(stream, "{:<12}", "DOUBLE")?;
                    stream.set_color(&NO_COLOR)?;
                    write!(stream, "{:.5}", d)?;
                }
                KOSValue::String(s) => {
                    write!(stream, "{:<12.80}", "STRING")?;
                    stream.set_color(&NO_COLOR)?;
                    write!(stream, "\"")?;
                    if s.starts_with('$') {
                        stream.set_color(&LIGHT_RED)?;
                    } else {
                        stream.set_color(&NO_COLOR)?;
                    }
                    write!(stream, "{}", s)?;
                    stream.set_color(&NO_COLOR)?;
                    write!(stream, "\"")?;
                }
                KOSValue::ArgMarker => {
                    write!(stream, "{:<12}", "ARGMARKER")?;
                    stream.set_color(&NO_COLOR)?;
                }
                KOSValue::ScalarInt(i) => {
                    write!(stream, "{:<12}", "SCALARINT")?;
                    stream.set_color(&NO_COLOR)?;
                    write!(stream, "{}", i)?;
                }
                KOSValue::ScalarDouble(d) => {
                    write!(stream, "{:<12}", "SCALARDOUBLE")?;
                    stream.set_color(&NO_COLOR)?;
                    write!(stream, "{}", d)?;
                }
                KOSValue::BoolValue(b) => {
                    write!(stream, "{:<12}", "SCALARDOUBLE")?;
                    stream.set_color(&NO_COLOR)?;
                    write!(stream, "{}", if *b { "true" } else { "false" })?;
                }
                KOSValue::StringValue(s) => {
                    write!(stream, "{:<12.80}", "STRINGVALUE")?;
                    if s.starts_with('$') {
                        stream.set_color(&LIGHT_RED)?;
                    } else {
                        stream.set_color(&NO_COLOR)?;
                    }
                    write!(stream, "\"{}\"", s)?;
                }
            }
            writeln!(stream)?;
        }

        Ok(())
    }
}

fn get_info(value: Option<&KOSValue>) -> String {
    match value {
        Some(value) => {
            match value {
                KOSValue::String(s) => {
                    // If it is either a label that is used for reset or a KS formatted function name
                    if s.starts_with('@') || s.contains('`') {
                        String::from("Compiled using official kOS compiler.")
                    } else {
                        s.to_string()
                    }
                }
                _ => String::from("Unknown compiler 2"),
            }
        }
        None => String::from("Unknown compiler"),
    }
}

#[cfg(test)]
mod tests {
    use crate::output::ksm::get_info;
    use kerbalobjects::KOSValue;

    #[test]
    fn official_info() {
        let value = KOSValue::String(String::from("@0001"));
        assert_eq!(
            get_info(Some(&value)),
            String::from("Compiled using official kOS compiler.")
        );
    }

    #[test]
    fn arbitrary_info() {
        let info = String::from("My favorite compiler");
        let value = KOSValue::String(info.clone());
        assert_eq!(get_info(Some(&value)), info);
    }
}
