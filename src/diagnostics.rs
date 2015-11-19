use std::fmt;
use ansi_term::Colour::{Red,Purple};
use ansi_term::Style;
use self::Level::*;

#[derive(Debug)]
#[allow(dead_code)]
pub enum Level {
    Warning,
    Error
}

#[derive(Debug)]
pub struct Info {
    pub level: Level,
    pub filename: String,
    pub message: String,
}

impl fmt::Display for Info {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        let bold = Style::new().bold();
        match self.level {
            Warning => {
                write!(f, "{}: {} {}", self.filename,
                       Purple.paint("warning:").to_string(),
                       bold.paint(self.message.clone()).to_string())
            }
            Error => {
                write!(f, "{}: {} {}", self.filename,
                       Red.paint("error:").to_string(),
                       bold.paint(self.message.clone()).to_string())
            }
        }
    }
}
