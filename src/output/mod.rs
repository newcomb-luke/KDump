use kerbalobjects::KOSValue;
use std::error::Error;
use std::io::Write;
use termcolor::ColorSpec;
use termcolor::StandardStream;
use termcolor::WriteColor;

type DynResult<T> = Result<T, Box<dyn Error>>;
type DumpResult = DynResult<()>;

mod ko;
pub use ko::KOFileDebug;

mod ksm;
pub use ksm::KSMFileDebug;

pub fn kosvalue_str(value: &KOSValue) -> String {
    let mut s = String::new();

    match value {
        kerbalobjects::KOSValue::Null => {
            s.push_str("#");
        }
        kerbalobjects::KOSValue::Bool(b) => {
            s.push_str(if *b { "true" } else { "false" });
        }
        kerbalobjects::KOSValue::Byte(b) => {
            s = format!("{}", b);
        }
        kerbalobjects::KOSValue::Int16(i) => {
            s = format!("{}", i);
        }
        kerbalobjects::KOSValue::Int32(i) => {
            s = format!("{}", i);
        }
        kerbalobjects::KOSValue::Float(f) => {
            s = format!("{:.5}", f);
        }
        kerbalobjects::KOSValue::Double(d) => {
            s = format!("{:.5}", d);
        }
        kerbalobjects::KOSValue::String(v) => {
            s = v.clone();
        }
        kerbalobjects::KOSValue::ArgMarker => {
            s.push_str("@");
        }
        kerbalobjects::KOSValue::ScalarInt(i) => {
            s = format!("{}", i);
        }
        kerbalobjects::KOSValue::ScalarDouble(d) => {
            s = format!("{:.5}", d);
        }
        kerbalobjects::KOSValue::BoolValue(b) => {
            s.push_str(if *b { "true" } else { "false" });
        }
        kerbalobjects::KOSValue::StringValue(v) => {
            s = v.clone();
        }
    }

    s
}

fn write_kosvalue(
    stream: &mut StandardStream,
    value: &KOSValue,
    regular_color: &ColorSpec,
    variable_color: &ColorSpec,
) -> DumpResult {
    let mut str_value = "";

    let is_string = match value {
        KOSValue::String(s) | KOSValue::StringValue(s) => {
            str_value = s;
            true
        }
        _ => false,
    };

    let is_variable = is_string && str_value.starts_with("$");

    if is_string {
        write!(stream, "\"")?;
    }

    if is_variable {
        stream.set_color(variable_color)?;
    }

    write!(stream, "{}", kosvalue_str(value))?;

    if is_string {
        stream.set_color(regular_color)?;
        write!(stream, "\"")?;
    }

    Ok(())
}
