use std::error::Error;

use crate::KSMFileReader;

pub struct Instr {
    opcode: u8,
    operand_width: u8,
    num_operands: u8,
    operands: Vec<u32>,
}

impl Instr {
    pub fn new(opcode: u8, operand_width: u8, num_operands: u8, operands: Vec<u32>) -> Instr {
        Instr {
            opcode,
            operand_width,
            num_operands,
            operands,
        }
    }

    pub fn get_mnemonic(&self) -> String {
        String::from(match self.opcode {
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

    pub fn opcode_num_operands(opcode: u8) -> u8 {
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
            _ => 255,
        }
    }

    pub fn size(&self) -> u8 {
        1 + self.operand_width * self.num_operands
    }

    pub fn num_operands(&self) -> usize {
        self.num_operands as usize
    }

    pub fn raw_str(&self) -> String {
        let mut raw = format!("{:02x} ", self.opcode);

        for operand in self.operands.iter() {
            raw.push_str(&format!(
                "{0:0width$x} ",
                operand,
                width = (self.operand_width * 2) as usize
            ));
        }

        format!(
            "{:<width$} ",
            raw,
            width = (self.operand_width * 6 + 3) as usize
        )
    }

    pub fn get_opcode(&self) -> u8 {
        self.opcode
    }

    pub fn get_operands(&self) -> &Vec<u32> {
        &self.operands
    }

    pub fn read(reader: &mut KSMFileReader) -> Result<Instr, Box<dyn Error>> {
        let opcode = reader.next()?;

        let num_operands = Instr::opcode_num_operands(opcode);

        let mut operands: Vec<u32> = Vec::with_capacity(num_operands as usize);

        for _ in 0..num_operands {
            operands.push(reader.read_argument_address()?);
        }

        Ok(Instr::new(
            opcode,
            reader.get_address_bytes(),
            num_operands,
            operands,
        ))
    }
}
