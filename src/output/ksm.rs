use crate::CLIConfig;
use crate::DARK_RED_COLOR;
use crate::GREEN_COLOR;
use crate::LIGHT_RED_COLOR;
use crate::PURPLE_COLOR;
use kerbalobjects::ksmfile::sections::CodeSection;
use kerbalobjects::ksmfile::KSMFile;
use kerbalobjects::KOSValue;
use kerbalobjects::Opcode;
use std::io::Write;
use termcolor::ColorSpec;
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
            writeln!(stream, "\nKSM File Info:")?;
            writeln!(stream, "\t{}", self.get_info())?;
        }

        if config.argument_section || config.full_contents {
            self.dump_argument_section(stream, &no_color, &green, &light_red)?;
        }

        if config.disassemble || config.full_contents {
            self.dump_code_sections(stream, config, &no_color, &purple, &dark_red, &light_red)?;
        }

        Ok(())
    }

    fn get_info(&self) -> String {
        match self.ksmfile.arg_section().get(0) {
            Some(value) => {
                match value {
                    KOSValue::String(s) => {
                        // If it is either a label that is used for reset or a KS formatted function name
                        if s.starts_with("@") || s.contains("`") {
                            String::from("Compiled using official kOS compiler.")
                        } else {
                            format!("{}", s)
                        }
                    }
                    _ => String::from("Unknown compiler"),
                }
            }
            None => String::from("Unknown compiler"),
        }
    }

    fn dump_code_sections(
        &self,
        stream: &mut StandardStream,
        config: &CLIConfig,
        regular_color: &ColorSpec,
        label_color: &ColorSpec,
        mnemonic_color: &ColorSpec,
        variable_color: &ColorSpec,
    ) -> DumpResult {
        let mut index = 1;

        for code_section in self.ksmfile.code_sections() {
            if code_section.instructions().len() != 0 {
                let new_index = self.dump_code_section(
                    stream,
                    code_section,
                    index,
                    regular_color,
                    label_color,
                    mnemonic_color,
                    variable_color,
                    !config.show_no_labels,
                    !config.show_no_raw_instr,
                )?;

                index = new_index;
            }
        }

        Ok(())
    }

    fn dump_code_section(
        &self,
        stream: &mut StandardStream,
        code_section: &CodeSection,
        start_index: i32,
        regular_color: &ColorSpec,
        label_color: &ColorSpec,
        mnemonic_color: &ColorSpec,
        variable_color: &ColorSpec,
        show_labels: bool,
        show_raw_instr: bool,
    ) -> DynResult<i32> {
        let section_type = code_section.section_type();
        let addr_width = self.ksmfile.arg_section().num_index_bytes();

        let name = match section_type {
            kerbalobjects::ksmfile::sections::CodeType::Main => "MAIN",
            kerbalobjects::ksmfile::sections::CodeType::Initialization => "INIT",
            kerbalobjects::ksmfile::sections::CodeType::Function => {
                match code_section.instructions().next() {
                    Some(instr) => match instr {
                        kerbalobjects::ksmfile::Instr::OneOp(opcode, op1) => {
                            if *opcode == Opcode::Lbrt {
                                let operand = self.value_from_operand(*op1)?;

                                match operand {
                                    KOSValue::String(s) | KOSValue::StringValue(s) => {
                                        // If this is a kOS-compiled function
                                        if s.contains("`") {
                                            s.split("`").next().unwrap()
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
                    },
                    None => "FUNC",
                }
            }
            kerbalobjects::ksmfile::sections::CodeType::Unknown => "UNKNOWN",
        };

        stream.set_color(regular_color)?;
        writeln!(stream, "\n{}:", name)?;

        let mut label = String::from("@0000001");
        let mut index = start_index;

        for instr in code_section.instructions() {
            let instr_opcode = match instr {
                kerbalobjects::ksmfile::Instr::ZeroOp(opcode) => *opcode,
                kerbalobjects::ksmfile::Instr::OneOp(opcode, _) => *opcode,
                kerbalobjects::ksmfile::Instr::TwoOp(opcode, _, _) => *opcode,
            };

            let is_lbrt = instr_opcode == Opcode::Lbrt;

            if show_labels {
                stream.set_color(label_color)?;

                if is_lbrt {
                    write!(stream, "  {:7} ", "")?;
                } else {
                    write!(stream, "  {:<7} ", label)?;
                }
            }

            stream.set_color(regular_color)?;

            if is_lbrt {
                match instr {
                    kerbalobjects::ksmfile::Instr::OneOp(_, op) => {
                        let arg = self.value_from_operand(*op)?;

                        match arg {
                            KOSValue::String(s) => {
                                label = s.clone();

                                if label.starts_with("@") {
                                    // Makes @0013 @000013
                                    label.insert_str(1, "00");
                                }
                            }
                            _ => {}
                        }

                        label.truncate(7);
                    }
                    _ => {}
                }
            }
            // If it isn't a label reset
            else {
                index += 1;
                label = format!("@{:>06}", index);
            }

            if show_raw_instr {
                match instr {
                    kerbalobjects::ksmfile::Instr::ZeroOp(opcode) => {
                        write!(
                            stream,
                            "{:x} {:<width$} {:<width$}",
                            u8::from(*opcode),
                            "",
                            "",
                            width = addr_width * 2
                        )?;
                    }
                    kerbalobjects::ksmfile::Instr::OneOp(opcode, op1) => {
                        write!(
                            stream,
                            "{:x} {:0>width$x} {:<width$}",
                            u8::from(*opcode),
                            op1,
                            "",
                            width = addr_width * 2
                        )?;
                    }
                    kerbalobjects::ksmfile::Instr::TwoOp(opcode, op1, op2) => {
                        write!(
                            stream,
                            "{:x} {:0>width$x} {:0>width$x}",
                            u8::from(*opcode),
                            op1,
                            op2,
                            width = addr_width * 2
                        )?;
                    }
                }
            }

            stream.set_color(mnemonic_color)?;

            let mnemonic: &str = instr_opcode.into();

            write!(stream, "  {:<6}", mnemonic)?;

            stream.set_color(regular_color)?;

            match instr {
                kerbalobjects::ksmfile::Instr::ZeroOp(_) => {}
                kerbalobjects::ksmfile::Instr::OneOp(_, op1) => {
                    let val1 = self.value_from_operand(*op1)?;

                    super::write_kosvalue(stream, val1, regular_color, variable_color)?;
                }
                kerbalobjects::ksmfile::Instr::TwoOp(_, op1, op2) => {
                    let val1 = self.value_from_operand(*op1)?;
                    let val2 = self.value_from_operand(*op2)?;

                    super::write_kosvalue(stream, val1, regular_color, variable_color)?;

                    write!(stream, ",")?;

                    super::write_kosvalue(stream, val2, regular_color, variable_color)?;
                }
            }

            writeln!(stream, "")?;
        }

        Ok(index)
    }

    fn value_from_operand(&self, op: usize) -> DynResult<&KOSValue> {
        self.ksmfile
            .arg_section()
            .get(op)
            .ok_or("Instruction referenced invalid argument index".into())
    }

    fn dump_argument_section(
        &self,
        stream: &mut StandardStream,
        regular_color: &ColorSpec,
        type_color: &ColorSpec,
        variable_color: &ColorSpec,
    ) -> DumpResult {
        let arg_section = self.ksmfile.arg_section();
        let addr_width = arg_section.num_index_bytes();

        stream.set_color(regular_color)?;

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
            stream.set_color(regular_color)?;

            let index_str = format!("  {:0>width$x}", index, width = addr_width * 2);

            write!(stream, "{:<20}", index_str)?;

            index += value.size_bytes();

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
                    write!(stream, "{:<12.80}", "STRING")?;
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
                    write!(stream, "{:<12.80}", "STRINGVALUE")?;
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

        Ok(())
    }
}
