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
        let mut info_line = self.filename.to_owned();

        // Find line and column offsets, if we have an index.
        let offsets = match (self.position, &self.source) {
            (Some((from, to)), &Some(ref source)) => {
                let (line_idx, column_idx) = position(source, from);

                info_line = format!("{}:{}:{}", info_line, line_idx + 1, column_idx + 1);
                Some((line_idx, column_idx, to - from))
            }
            _ => None
        };

        let color;
        match self.level {
            Warning => {
                color = Purple;
                info_line = format!("{} {}", info_line, color.paint("warning").to_string());
            }
            Error => {
                color = Red;
                info_line = format!("{} {}", info_line, color.paint("error").to_string());
            }
        }

        info_line = format!("{}: {}", info_line, self.message);

        let bold = Style::new().bold();
        try!(write!(f, "{}", bold.paint(info_line).to_string()));

        match (offsets, &self.source) {
            (Some((line_idx, column_idx, width)), &Some(ref source)) => {
                // Print the offending line.
                let line = source.split('\n').nth(line_idx).unwrap();
                try!(write!(f, "\n{}\n", line));

                // Highlight the bad characters on that line.
                let mut caret_line = "".to_owned();
                for _ in 0..column_idx {
                    caret_line = caret_line + " ";
                }
                caret_line = caret_line + "^";
                for _ in 0..width {
                    caret_line = caret_line + "~";
                }

                try!(write!(f, "{}", color.bold().paint(caret_line).to_string()));
            }
            _ => {}
        }

        Ok(())
    }
}
