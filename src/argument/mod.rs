use std::error::Error;

use crate::KSMFileReader;

pub enum Value {
    NULL,
    Boolean(bool),
    Byte(i8),
    Int16(i16),
    Int32(i32),
    Float(f32),
    Double(f64),
    String(String),
    ARGMARKER,
    ScalarIntValue(i32),
    ScalarDoubleValue(f64),
    BooleanValue(bool),
    StringValue(String),
}

static VALUE_TYPES: &'static [&'static str] = &[
    "NULL",
    "BOOL",
    "BYTE",
    "INT16",
    "INT32",
    "F32",
    "F64",
    "STRING",
    "ARGMARKER",
    "SCALARINT",
    "SCALARF64",
    "BOOLVALUE",
    "STRINGVALUE",
];

pub struct Argument {
    value_type: usize,
    address: u32,
    value: Value,
}

impl Argument {
    pub fn new(value_type: usize, address: u32, value: Value) -> Argument {
        Argument {
            value_type,
            address,
            value,
        }
    }

    pub fn get_repr(&self) -> String {
        match &self.value {
            Value::NULL => String::from("NULL"),
            Value::Boolean(b) => String::from(if *b { "true" } else { "false" }),
            Value::Byte(b) => format!("{:#x}", b),
            Value::Int16(i) => format!("{:#04x}", i),
            Value::Int32(i) => format!("{:#06x}", i),
            Value::Float(fl) => format!("{:.5}", fl),
            Value::Double(d) => format!("{:.5}", d),
            Value::String(s) => s.to_string(),
            Value::ARGMARKER => String::from("ARGM"),
            Value::ScalarIntValue(i) => format!("{:#06x}", i),
            Value::ScalarDoubleValue(d) => format!("{:.5}", d),
            Value::BooleanValue(b) => String::from(if *b { "true" } else { "false" }),
            Value::StringValue(s) => s.to_string(),
        }
    }

    pub fn is_variable(&self) -> bool {
        match &self.value {
            Value::String(s) | Value::StringValue(s) => s.starts_with('$'),
            _ => false,
        }
    }

    pub fn get_type_str(&self) -> String {
        String::from(VALUE_TYPES[self.value_type])
    }

    pub fn get_address(&self) -> u32 {
        self.address
    }

    pub fn get_value(&self) -> &Value {
        &self.value
    }

    pub fn get_type(&self) -> usize {
        self.value_type
    }

    pub fn read(reader: &mut KSMFileReader) -> Result<Argument, Box<dyn Error>> {
        let address = reader.get_current_index() - 4;

        let value_type: usize = reader.next()? as usize;

        let value = match value_type {
            0 => Value::NULL,
            1 => Value::Boolean(reader.read_boolean()?),
            2 => Value::Byte(reader.read_byte()?),
            3 => Value::Int16(reader.read_int16()?),
            4 => Value::Int32(reader.read_int32()?),
            5 => Value::Float(reader.read_float()?),
            6 => Value::Double(reader.read_double()?),
            7 => Value::String(reader.read_string()?),
            8 => Value::ARGMARKER,
            9 => Value::ScalarIntValue(reader.read_int32()?),
            10 => Value::ScalarDoubleValue(reader.read_double()?),
            11 => Value::BooleanValue(reader.read_boolean()?),
            12 => Value::StringValue(reader.read_string()?),
            _ => return Err(format!("Unknown argument type encountered: {:x}", value_type).into()),
        };

        // let (value, len) = match value_type {
        //     0 => (Value::NULL, 1),
        //     1 => (Value::Boolean(reader.read_boolean()?), 2),
        //     2 => (Value::Byte(reader.read_byte()?), 2),
        //     3 => (Value::Int16(reader.read_int16()?), 3),
        //     4 => (Value::Int32(reader.read_int32()?), 4),
        //     5 => (Value::Float(reader.read_float()?), 4),
        //     6 => (Value::Double(reader.read_double()?), 8),
        //     7 => (Value::String(reader.read_string()?), 1),
        //     8 => (Value::ARGMARKER, 1),
        //     9 => (Value::ScalarIntValue(reader.read_int32()?), 4),
        //     10 => (Value::ScalarDoubleValue(reader.read_double()?), 8),
        //     11 => (Value::BooleanValue(reader.read_boolean()?), 2),
        //     12 => (Value::StringValue(reader.read_string()?), 1),
        //     _ => return Err(format!("Unkown argument type encountered: {}", value_type).into())
        // };

        // // Add the length of the string if the value is a string type
        // len += match value {
        //     Value::String(s) => s.len() as u32,
        //     Value::StringValue(s) => s.len() as u32,
        //     _ => 0
        // };

        // Ok( (Argument::new(value_type, address as u32, value), len) )

        Ok(Argument::new(value_type, address as u32, value))
    }
}
