use std::fmt;

use self::Level::*;

#[derive(Debug)]
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
        // TODO: bold/colour like clang.
        match self.level {
            Warning => {
                write!(f, "{}: warning: {}", self.filename, self.message)
            }
            Error => {
                write!(f, "{}: error: {}", self.filename, self.message)
            }
        }
    }
}
