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
    pub source: Option<String>,
}

// Given an index into a string, return the line number and column
// count (both zero-indexed).
fn position(s: &str, i: usize) -> (usize, usize) {
    let mut char_count = 0;
    for (line_idx, line) in s.split('\n').enumerate() {
        let line_length = line.len();
        if char_count + line_length >= i {
            return (line_idx, i - char_count);
        }

        char_count += line_length + 1;
    }

    unreachable!()
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

        match (self.position, &self.source) {
            (Some((from,to)), &Some(ref source)) => {
                let (line_idx, column_idx) = position(source, from);

                // Print the offending line.
                let line = source.split('\n').nth(line_idx).unwrap();
                try!(write!(f, "\n{}\n", line));

                // Highlight the bad characters on that line.
                for _ in 0..column_idx {
                    try!(write!(f, " "));
                }
                try!(write!(f, "^"));
                for _ in 0..(to - from) {
                    try!(write!(f, "~"));
                }
            }
            _ => {}
        }

        Ok(())
    }
}
