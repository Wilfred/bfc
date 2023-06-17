//! bfir defines an AST for BF. This datastructure represents the
//! original BF source code with position data so we can find the
//! source lines from a portion of AST.
//!
//! It also provides functions for generating ASTs from source code,
//! producing good error messages on malformed inputs.

use std::collections::HashMap;
use std::fmt;
use std::num::Wrapping;

use self::AstNode::*;

/// `BfValue` represents the size of a single BF cell. BF requires
/// this to be at least one byte, we provide a BF cell of exactly one
/// byte.
pub type BfValue = Wrapping<i8>;

/// An inclusive range used for tracking positions in source code.
#[derive(PartialEq, Eq, Clone, Copy)]
pub struct Position {
    pub start: usize,
    pub end: usize,
}

impl fmt::Debug for Position {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if self.start == self.end {
            write!(f, "{}", self.start)
        } else {
            write!(f, "{}-{}", self.start, self.end)
        }
    }
}

pub trait Combine<T> {
    fn combine(&self, _: T) -> T;
}

impl Combine<Option<Position>> for Option<Position> {
    fn combine(&self, other: Self) -> Self {
        match (*self, other) {
            (Some(pos1), Some(pos2)) => {
                let (first_pos, second_pos) = if pos1.start <= pos2.start {
                    (pos1, pos2)
                } else {
                    (pos2, pos1)
                };

                // If they're adjacent positions, we can merge them.
                if first_pos.end + 1 >= second_pos.start {
                    Some(Position {
                        start: first_pos.start,
                        end: second_pos.end,
                    })
                } else {
                    // Otherwise, just use the second position.
                    Some(pos2)
                }
            }
            _ => None,
        }
    }
}

/// `AstNode` represents a node in our BF AST.
#[derive(PartialEq, Eq, Debug, Clone)]
pub enum AstNode {
    /// The `+` and `-` instructions in BF.
    Increment {
        /// The amount to increment the current BF cell by.
        ///
        /// `amount` can have values other than 1 and -1 after
        /// simplification, e.g. `++` is simplified to a single
        /// Increment with an amount of 2.
        amount: BfValue,
        /// The offset of the affected BF cell relative to the current
        /// BF cell pointer.
        ///
        /// For example, `>+<` is an increment at offset 1.
        offset: isize,
        position: Option<Position>,
    },
    /// The `<` and `>` instructions in BF. `amount` may not be 1 or
    /// -1 after simplifying e.g. `>>`.
    PointerIncrement {
        amount: isize,
        position: Option<Position>,
    },
    /// The `,` instruction in BF.
    Read { position: Option<Position> },
    /// The `.` instruction in BF.
    Write { position: Option<Position> },
    /// A loop in BF, such as `[>]`.
    Loop {
        body: Vec<AstNode>,
        position: Option<Position>,
    },
    /// Set the current BF cell to a value.
    ///
    /// This is only emitted during simplification. For example, `[-]`
    /// is equivalent to setting the BF cell to zero.
    Set {
        amount: BfValue,
        /// The offset of the affected BF cell relative to the current
        /// BF cell pointer.
        ///
        /// For example, `>[-]<` is `Set { amount: 0, offset: 1}`.
        offset: isize,
        position: Option<Position>,
    },
    /// Assign multiple other BF cells to the value of this cell,
    /// multiplied by a constant. This also sets the current BF cell
    /// to zero.
    ///
    /// For example, `[>+++<-]` is `MultiplyMove { changes: { 1: 3 }}`.
    MultiplyMove {
        changes: HashMap<isize, BfValue>,
        position: Option<Position>,
    },
}

fn fmt_with_indent(instr: &AstNode, indent: i32, f: &mut fmt::Formatter) {
    for _ in 0..indent {
        let _ = write!(f, "  ");
    }

    match instr {
        &Loop {
            body: ref loop_body,
            position,
            ..
        } => {
            let _ = write!(f, "Loop position: {:?}", position);

            for loop_instr in loop_body {
                let _ = writeln!(f);
                fmt_with_indent(loop_instr, indent + 1, f);
            }
        }
        instr => {
            let _ = write!(f, "{:?}", instr);
        }
    }
}

impl fmt::Display for AstNode {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt_with_indent(self, 0, f);
        Ok(())
    }
}

pub fn get_position(instr: &AstNode) -> Option<Position> {
    match *instr {
        Increment { position, .. } => position,
        PointerIncrement { position, .. } => position,
        Read { position } => position,
        Write { position } => position,
        Loop { position, .. } => position,
        Set { position, .. } => position,
        MultiplyMove { position, .. } => position,
    }
}

#[derive(Debug)]
pub struct ParseError {
    pub message: String,
    pub position: Position,
}

/// Given a string of BF source code, parse and return our BF IR
/// representation. If parsing fails, return a position and message
/// describing what went wrong.
pub fn parse(source: &str) -> Result<Vec<AstNode>, ParseError> {
    // AstNodes in the current loop (or toplevel).
    let mut instructions = vec![];
    // Contains the instructions of open parent loops (or toplevel),
    // and the starting indices of the loops.
    let mut stack = vec![];

    for (index, c) in source.chars().enumerate() {
        match c {
            '+' => instructions.push(Increment {
                amount: Wrapping(1),
                offset: 0,
                position: Some(Position {
                    start: index,
                    end: index,
                }),
            }),
            '-' => instructions.push(Increment {
                amount: Wrapping(-1),
                offset: 0,
                position: Some(Position {
                    start: index,
                    end: index,
                }),
            }),
            '>' => instructions.push(PointerIncrement {
                amount: 1,
                position: Some(Position {
                    start: index,
                    end: index,
                }),
            }),
            '<' => instructions.push(PointerIncrement {
                amount: -1,
                position: Some(Position {
                    start: index,
                    end: index,
                }),
            }),
            ',' => instructions.push(Read {
                position: Some(Position {
                    start: index,
                    end: index,
                }),
            }),
            '.' => instructions.push(Write {
                position: Some(Position {
                    start: index,
                    end: index,
                }),
            }),
            '[' => {
                stack.push((instructions, index));
                instructions = vec![];
            }
            ']' => {
                if let Some((mut parent_instr, open_index)) = stack.pop() {
                    parent_instr.push(Loop {
                        body: instructions,
                        position: Some(Position {
                            start: open_index,
                            end: index,
                        }),
                    });
                    instructions = parent_instr;
                } else {
                    return Err(ParseError {
                        message: "This ] has no matching [".to_owned(),
                        position: Position {
                            start: index,
                            end: index,
                        },
                    });
                }
            }
            _ => (),
        }
    }

    if !stack.is_empty() {
        let pos = stack.last().unwrap().1;
        return Err(ParseError {
            message: "This [ has no matching ]".to_owned(),
            position: Position {
                start: pos,
                end: pos,
            },
        });
    }

    Ok(instructions)
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn parse_increment() {
        assert_eq!(
            parse("+").unwrap(),
            [Increment {
                amount: Wrapping(1),
                offset: 0,
                position: Some(Position { start: 0, end: 0 }),
            }]
        );
        assert_eq!(
            parse("++").unwrap(),
            [
                Increment {
                    amount: Wrapping(1),
                    offset: 0,
                    position: Some(Position { start: 0, end: 0 }),
                },
                Increment {
                    amount: Wrapping(1),
                    offset: 0,
                    position: Some(Position { start: 1, end: 1 }),
                }
            ]
        );
    }

    #[test]
    fn parse_decrement() {
        assert_eq!(
            parse("-").unwrap(),
            [Increment {
                amount: Wrapping(-1),
                offset: 0,
                position: Some(Position { start: 0, end: 0 }),
            }]
        );
    }

    #[test]
    fn parse_pointer_increment() {
        assert_eq!(
            parse(">").unwrap(),
            [PointerIncrement {
                amount: 1,
                position: Some(Position { start: 0, end: 0 }),
            }]
        );
    }

    #[test]
    fn parse_pointer_decrement() {
        assert_eq!(
            parse("<").unwrap(),
            [PointerIncrement {
                amount: -1,
                position: Some(Position { start: 0, end: 0 }),
            }]
        );
    }

    #[test]
    fn parse_read() {
        assert_eq!(
            parse(",").unwrap(),
            [Read {
                position: Some(Position { start: 0, end: 0 })
            }]
        );
    }

    #[test]
    fn parse_write() {
        assert_eq!(
            parse(".").unwrap(),
            [Write {
                position: Some(Position { start: 0, end: 0 })
            }]
        );
    }

    #[test]
    fn parse_empty_loop() {
        let expected = [Loop {
            body: vec![],
            position: Some(Position { start: 0, end: 1 }),
        }];
        assert_eq!(parse("[]").unwrap(), expected);
    }

    #[test]
    fn parse_simple_loop() {
        let loop_body = vec![Increment {
            amount: Wrapping(1),
            offset: 0,
            position: Some(Position { start: 1, end: 1 }),
        }];
        let expected = [Loop {
            body: loop_body,
            position: Some(Position { start: 0, end: 2 }),
        }];
        assert_eq!(parse("[+]").unwrap(), expected);
    }

    #[test]
    fn parse_complex_loop() {
        let loop_body = vec![
            Read {
                position: Some(Position { start: 2, end: 2 }),
            },
            Increment {
                amount: Wrapping(1),
                offset: 0,
                position: Some(Position { start: 3, end: 3 }),
            },
        ];
        let expected = [
            Write {
                position: Some(Position { start: 0, end: 0 }),
            },
            Loop {
                body: loop_body,
                position: Some(Position { start: 1, end: 4 }),
            },
            Increment {
                amount: Wrapping(-1),
                offset: 0,
                position: Some(Position { start: 5, end: 5 }),
            },
        ];
        assert_eq!(parse(".[,+]-").unwrap(), expected);
    }

    #[test]
    fn parse_unbalanced_loop() {
        assert!(parse("[").is_err());
        assert!(parse("]").is_err());
        assert!(parse("][").is_err());
        assert!(parse("[][").is_err());
    }

    #[test]
    fn parse_comment() {
        assert_eq!(parse("foo! ").unwrap(), []);
    }

    #[test]
    fn test_combine_pos() {
        let pos1 = Some(Position { start: 1, end: 2 });
        let pos2 = Some(Position { start: 3, end: 4 });

        assert_eq!(pos1.combine(pos2), Some(Position { start: 1, end: 4 }));
    }

    #[test]
    fn test_combine_order() {
        let pos1 = Some(Position { start: 3, end: 4 });
        let pos2 = Some(Position { start: 1, end: 2 });

        assert_eq!(pos1.combine(pos2), Some(Position { start: 1, end: 4 }));
    }

    #[test]
    fn test_combine_pos_not_consecutive() {
        let pos1 = Some(Position { start: 1, end: 2 });
        let pos2 = Some(Position { start: 4, end: 5 });

        assert_eq!(pos1.combine(pos2), Some(Position { start: 4, end: 5 }));
    }

    #[test]
    fn test_combine_pos_overlap() {
        let pos1 = Some(Position { start: 1, end: 1 });
        let pos2 = Some(Position { start: 1, end: 3 });

        assert_eq!(pos1.combine(pos2), Some(Position { start: 1, end: 3 }));
    }
}
