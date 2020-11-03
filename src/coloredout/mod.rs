use std::error::Error;
use std::io::Write;
use termcolor::{ColorChoice, ColorSpec, StandardStream, WriteColor};

pub struct Terminal {
    stream: StandardStream,
    default_color: ColorSpec,
}

impl Terminal {
    pub fn new(default_color: ColorSpec) -> Terminal {
        let stream = StandardStream::stdout(ColorChoice::Auto);

        Terminal {
            stream,
            default_color,
        }
    }

    pub fn write(&mut self, text: &String) -> Result<(), Box<dyn Error>> {
        write!(&mut self.stream, "{}", text)?;

        Ok(())
    }

    pub fn writeln(&mut self, text: &String) -> Result<(), Box<dyn Error>> {
        writeln!(&mut self.stream, "{}", text)?;

        Ok(())
    }

    pub fn write_colored(
        &mut self,
        text: &String,
        color: &ColorSpec,
    ) -> Result<(), Box<dyn Error>> {
        self.stream.set_color(color)?;

        write!(&mut self.stream, "{}", text)?;

        self.stream.set_color(&self.default_color)?;

        Ok(())
    }

    pub fn writeln_colored(
        &mut self,
        text: &String,
        color: &ColorSpec,
    ) -> Result<(), Box<dyn Error>> {
        self.stream.set_color(color)?;

        writeln!(&mut self.stream, "{}", text)?;

        self.stream.set_color(&self.default_color)?;

        Ok(())
    }
}
