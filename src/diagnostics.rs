//! Human-readable warnings and errors for the CLI.

use ansi_term::ANSIStrings;
use ansi_term::Colour::{Purple, Red};
use ansi_term::Style;
use std::fmt;

use bfir::Position;

#[derive(Debug, PartialEq, Eq)]
pub struct Warning {
    pub message: String,
    pub position: Option<Position>,
}

/// The severity of the Info.
#[derive(Debug)]
#[allow(dead_code)]
pub enum Level {
    Warning,
    Error,
}

/// Info represents a message to the user, a warning or an error with
/// an optional reference to a position in the BF source.
#[derive(Debug)]
pub struct Info {
    pub level: Level,
    pub filename: String,
    pub message: String,
    pub position: Option<Position>,
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
        let mut file_text = self.filename.to_owned();

        // Find line and column offsets, if we have an index.
        let offsets = match (&self.position, &self.source) {
            (&Some(range), &Some(ref source)) => {
                debug_assert!(range.start <= range.end);

                let (line_idx, column_idx) = position(source, range.start);

                file_text = file_text + &format!(":{}:{}", line_idx + 1, column_idx + 1);
                Some((line_idx, column_idx, range.end - range.start))
            }
            _ => None,
        };

        let level_text;
        let color;
        match self.level {
            Level::Warning => {
                color = Purple;
                level_text = " warning: ";
            }
            Level::Error => {
                color = Red;
                level_text = " error: ";
            }
        }

        let mut context_line = "".to_owned();
        let mut caret_line = "".to_owned();
        if let (Some((line_idx, column_idx, width)), &Some(ref source)) = (offsets, &self.source) {
            // The faulty line of code.
            let line = source.split('\n').nth(line_idx).unwrap();
            context_line = "\n".to_owned() + line;

            // Highlight the faulty characters on that line.
            caret_line += "\n";
            for _ in 0..column_idx {
                caret_line += " ";
            }
            caret_line += "^";
            if width > 0 {
                for _ in 0..width {
                    caret_line += "~";
                }
            }
        }

        let bold = Style::new().bold();
        let default = Style::default();
        let strings = [
            bold.paint(file_text),
            color.bold().paint(level_text),
            bold.paint(self.message.clone()),
            default.paint(context_line),
            color.bold().paint(caret_line),
        ];
        write!(f, "{}", ANSIStrings(&strings))
    }
}
