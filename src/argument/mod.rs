use std::{error::Error, iter::Peekable, slice::Iter, fmt};

pub enum Argument {
    NULL,
    Boolean(bool),
    Byte(i8),
    Int16(i16),
    Int32(i32),
    Float(f32),
    Double(f64),
    String(String),
    ArgMarker,
    ScalarIntValue(i32),
    ScalarDoubleValue(f64),
    BooleanValue(bool),
    StringValue(String)
}

impl fmt::Display for Argument {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Argument::NULL => write!(f, "NULL"),
            Argument::Boolean(b) => write!(f, "{}", if *b { "true"} else { "false"}),
            Argument::Byte(b) => write!(f, "{:x?}", b),
            Argument::Int16(i) => write!(f, "{:x?}", i),
            Argument::Int32(i) => write!(f, "{:x?}", i),
            Argument::Float(fl) => write!(f, "{:.5}", fl),
            Argument::Double(d) => write!(f, "{:.5}", d),
            Argument::String(s) => write!(f, "{}", s),
            Argument::ArgMarker => write!(f, "ARGM"),
            Argument::ScalarIntValue(i) => write!(f, "{:x?}", i),
            Argument::ScalarDoubleValue(d) => write!(f, "{:.5}", d),
            Argument::BooleanValue(b) => write!(f, "{}", if *b { "true"} else { "false"}),
            Argument::StringValue(s) => write!(f, "{}", s)
        }
    }
}

pub fn argument_type_string(arg: &Argument) -> String {
    match *arg {
        Argument::NULL => String::from("NULL"),
        Argument::Boolean(_) => String::from("BOOL"),
        Argument::Byte(_) => String::from("BYTE"),
        Argument::Int16(_) => String::from("INT16"),
        Argument::Int32(_) => String::from("INT32"),
        Argument::Float(_) => String::from("F32"),
        Argument::Double(_) => String::from("F64"),
        Argument::String(_) => String::from("STRING"),
        Argument::ArgMarker => String::from("ARGMARKER"),
        Argument::ScalarIntValue(_) => String::from("SCALARINT"),
        Argument::ScalarDoubleValue(_) => String::from("SCALARF64"),
        Argument::BooleanValue(_) => String::from("BOOLVALUE"),
        Argument::StringValue(_) => String::from("STRINGVALUE")
    }
}

fn read_boolean(byte_iter: &mut Peekable<Iter<u8>>) -> Option<Argument> {
    match byte_iter.next() {
        Some(v) => Some(Argument::Boolean(*v != 0u8)),
        None => None
    }
}

fn read_boolean_value(byte_iter: &mut Peekable<Iter<u8>>) -> Option<Argument> {
    match byte_iter.next() {
        Some(v) => Some(Argument::BooleanValue(*v != 0u8)),
        None => None
    }
}

fn read_byte(byte_iter: &mut Peekable<Iter<u8>>) -> Option<Argument> {
    match byte_iter.next() {
        Some(v) => Some(Argument::Byte(*v as i8)),
        None => None
    }
}

fn read_int16(byte_iter: &mut Peekable<Iter<u8>>) -> Option<Argument> {
    let mut arr: [u8; 2] = [0; 2];

    for i in 0..2 {
        match byte_iter.next() {
            Some(v) => arr[i] = *v,
            None => return None
        }
    }

    return Some(Argument::Int16( i16::from_le_bytes(arr) ));
}

fn read_int32(byte_iter: &mut Peekable<Iter<u8>>) -> Option<Argument> {
    let mut arr: [u8; 4] = [0; 4];

    for i in 0..4 {
        match byte_iter.next() {
            Some(v) => arr[i] = *v,
            None => return None
        }
    }

    return Some(Argument::Int32( i32::from_le_bytes(arr) ));
}

fn read_scalar_int_value(byte_iter: &mut Peekable<Iter<u8>>) -> Option<Argument> {
    let mut arr: [u8; 4] = [0; 4];

    for i in 0..4 {
        match byte_iter.next() {
            Some(v) => arr[i] = *v,
            None => return None
        }
    }

    return Some(Argument::ScalarIntValue( i32::from_le_bytes(arr) ));
}

fn read_float(byte_iter: &mut Peekable<Iter<u8>>) -> Option<Argument> {
    let mut arr: [u8; 4] = [0; 4];

    for i in 0..4 {
        match byte_iter.next() {
            Some(v) => arr[i] = *v,
            None => return None
        }
    }

    return Some(Argument::Float( f32::from_le_bytes(arr) ));
}

fn read_double(byte_iter: &mut Peekable<Iter<u8>>) -> Option<Argument> {
    let mut arr: [u8; 8] = [0; 8];

    for i in 0..8 {
        match byte_iter.next() {
            Some(v) => arr[i] = *v,
            None => return None
        }
    }

    return Some(Argument::Double( f64::from_le_bytes(arr) ));
}

fn read_scalar_double_value(byte_iter: &mut Peekable<Iter<u8>>) -> Option<Argument> {
    let mut arr: [u8; 8] = [0; 8];

    for i in 0..8 {
        match byte_iter.next() {
            Some(v) => arr[i] = *v,
            None => return None
        }
    }

    return Some(Argument::ScalarDoubleValue( f64::from_le_bytes(arr) ));
}

fn read_string(byte_iter: &mut Peekable<Iter<u8>>) -> Option<Argument> {
    let len = match byte_iter.next() {
        Some(v) => *v,
        None => return None
    };

    let mut internal = String::with_capacity(len as usize);

    for _ in 0..len {
        match byte_iter.next() {
            Some(v) => internal.push( *v as char ),
            None => return None
        }
    }

    return Some(Argument::String(internal));
}

fn read_string_value(byte_iter: &mut Peekable<Iter<u8>>) -> Option<Argument> {
    match read_string(byte_iter) {
        Some(v) => {
            match v {
                Argument::String(s) => Some(Argument::StringValue(s)),
                _ => None
            }
        }
        None => None
    }
}

pub fn read_argument(byte_iter: &mut Peekable<Iter<u8>>) -> Result<(Argument, i32), Box<dyn Error>> {

    let arg_type = match byte_iter.next() {
        Some(v) => v,
        None => return Err("Reached EOF before the argument section eneded".into())
    };
    
    let mut argument_len: i32 = 0;

    let possible_argument = match arg_type {
        0 => Some(Argument::NULL),
        1 => { argument_len = 1; read_boolean(byte_iter)},
        2 => { argument_len = 1; read_byte(byte_iter)},
        3 => { argument_len = 2; read_int16(byte_iter)},
        4 => { argument_len = 4; read_int32(byte_iter)},
        5 => { argument_len = 4; read_float(byte_iter)},
        6 => { argument_len = 8; read_double(byte_iter)},
        7 => read_string(byte_iter),
        8 => Some(Argument::ArgMarker),
        9 => { argument_len = 4; read_scalar_int_value(byte_iter)},
        10 => { argument_len = 8; read_scalar_double_value(byte_iter)},
        11 => { argument_len = 1; read_boolean_value(byte_iter)},
        12 => read_string_value(byte_iter),
        _ => {
            return Err("Unrecognized argument type encountered in section. Is the file corrupt or this tool outdated?".into())
        }
    };

    match possible_argument {
        Some(v) => {

            match &v {
                Argument::String(s) => {
                    argument_len += s.len() as i32;
                },
                Argument::StringValue(s) => {
                    argument_len += s.len() as i32;
                },
                _ => ()
            };

            Ok((v, argument_len))
        },
        None => return Err("Reached EOF before the argument section eneded".into())
    }
}