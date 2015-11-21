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
    // from and to (can be the same)
    pub position: Option<(usize, usize)>,
}

impl fmt::Display for Info {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        try!(write!(f, "{}: ", self.filename));
        
        match self.level {
            Warning => {
                try!(write!(f, "{} ", Purple.paint("warning:").to_string()));
            }
            Error => {
                try!(write!(f, "{} ", Red.paint("error:").to_string()));
            }
        }

        let bold = Style::new().bold();
        try!(write!(f, "{}", bold.paint(self.message.clone()).to_string()));

        Ok(())
    }
}
