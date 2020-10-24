#![warn(trivial_numeric_casts)]

//! Calculate the maximum cell accessed by a BF program.

#[cfg(test)]
use pretty_assertions::assert_eq;
#[cfg(test)]
use quickcheck::quickcheck;
#[cfg(test)]
use std::collections::HashMap;
#[cfg(test)]
use std::num::Wrapping;

use std::cmp::{max, Ord, Ordering};
use std::ops::Add;

use crate::bfir::AstNode;
use crate::bfir::AstNode::*;

#[cfg(test)]
use crate::bfir::{parse, Position};

// 100,000 cells, zero-indexed.
pub const MAX_CELL_INDEX: usize = 99999;

/// Return the highest cell index that can be reached during program
/// execution. Zero-indexed.
pub fn highest_cell_index(instrs: &[AstNode]) -> usize {
    let (highest_index, _) = overall_movement(instrs);

    match highest_index {
        SaturatingInt::Number(x) => {
            if x > MAX_CELL_INDEX as i64 {
                // TODO: generate a warning here.
                MAX_CELL_INDEX
            } else {
                x as usize
            }
        }
        SaturatingInt::Max => MAX_CELL_INDEX,
    }
}

/// Saturating arithmetic: we have normal integers that work as
/// expected, but Max is bigger than any Number.
#[derive(Eq, PartialEq, Clone, Copy, Debug)]
enum SaturatingInt {
    Number(i64),
    Max,
}

impl Add for SaturatingInt {
    type Output = SaturatingInt;
    fn add(self, rhs: SaturatingInt) -> SaturatingInt {
        if let (&SaturatingInt::Number(x), &SaturatingInt::Number(y)) = (&self, &rhs) {
            SaturatingInt::Number(x + y)
        } else {
            SaturatingInt::Max
        }
    }
}

impl Ord for SaturatingInt {
    fn cmp(&self, other: &SaturatingInt) -> Ordering {
        match (self, other) {
            (&SaturatingInt::Max, &SaturatingInt::Max) => Ordering::Equal,
            (&SaturatingInt::Number(_), &SaturatingInt::Max) => Ordering::Less,
            (&SaturatingInt::Max, &SaturatingInt::Number(_)) => Ordering::Greater,
            (&SaturatingInt::Number(x), &SaturatingInt::Number(y)) => x.cmp(&y),
        }
    }
}

impl PartialOrd for SaturatingInt {
    fn partial_cmp(&self, other: &SaturatingInt) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// Return a tuple (highest cell index reached, cell index at end).
/// If movement is unbounded, return Max.
fn overall_movement(instrs: &[AstNode]) -> (SaturatingInt, SaturatingInt) {
    let mut net_movement = SaturatingInt::Number(0);
    let mut max_index = SaturatingInt::Number(0);

    for (instr_highest_offset, instr_net_movement) in instrs.iter().map(movement) {
        max_index = max(
            net_movement,
            max(net_movement + instr_highest_offset, max_index),
        );
        net_movement = net_movement + instr_net_movement;
    }
    (max_index, net_movement)
}

/// Return a tuple (highest cell index reached, cell index at end).
/// If movement is unbounded, return Max.
fn movement(instr: &AstNode) -> (SaturatingInt, SaturatingInt) {
    match *instr {
        PointerIncrement { amount, .. } => {
            if amount < 0 {
                (
                    SaturatingInt::Number(0),
                    SaturatingInt::Number(amount as i64),
                )
            } else {
                (
                    SaturatingInt::Number(amount as i64),
                    SaturatingInt::Number(amount as i64),
                )
            }
        }
        Increment { offset, .. } | Set { offset, .. } => (
            SaturatingInt::Number(offset as i64),
            SaturatingInt::Number(0),
        ),
        MultiplyMove { ref changes, .. } => {
            let mut highest_affected = 0;
            for cell in changes.keys() {
                if *cell > highest_affected {
                    highest_affected = *cell;
                }
            }
            (
                SaturatingInt::Number(highest_affected as i64),
                SaturatingInt::Number(0),
            )
        }
        Loop { ref body, .. } => {
            let (max_in_body, net_in_body) = overall_movement(body);

            match net_in_body {
                SaturatingInt::Number(net_loop_movement) => {
                    if net_loop_movement == 0 {
                        (max_in_body, SaturatingInt::Number(0))
                    } else if net_loop_movement < 0 {
                        // Net movement was negative, so conservatively assume
                        // it was zero (e.g. the loop may run zero times).
                        (max_in_body, SaturatingInt::Number(0))
                    } else {
                        // Net loop movement was positive, so we can't
                        // assume any bounds.
                        (SaturatingInt::Max, SaturatingInt::Max)
                    }
                }
                SaturatingInt::Max => {
                    // Unbounded movement somewhere inside the loop,
                    // so this loop is unbounded.
                    (SaturatingInt::Max, SaturatingInt::Max)
                }
            }
        }
        Read { .. } | Write { .. } => (SaturatingInt::Number(0), SaturatingInt::Number(0)),
    }
}

#[test]
fn one_cell_bounds() {
    let instrs = parse("+-.,").unwrap();
    assert_eq!(highest_cell_index(&instrs), 0);
}

#[test]
fn ptr_increment_bounds() {
    let instrs = parse(">").unwrap();
    assert_eq!(highest_cell_index(&instrs), 1);
}

#[test]
fn ptr_increment_sequence_bounds() {
    let instrs = parse(">>.<").unwrap();
    assert_eq!(highest_cell_index(&instrs), 2);

    let instrs = parse(">><>>").unwrap();
    assert_eq!(highest_cell_index(&instrs), 3);
}

#[test]
fn multiple_ptr_increment_bounds() {
    let instrs = vec![PointerIncrement {
        amount: 2,
        position: Some(Position { start: 0, end: 0 }),
    }];
    assert_eq!(highest_cell_index(&instrs), 2);
}

#[test]
fn multiply_move_bounds() {
    let mut dest_cells = HashMap::new();
    dest_cells.insert(1, Wrapping(3));
    dest_cells.insert(4, Wrapping(1));
    let instrs = vec![
        MultiplyMove {
            changes: dest_cells,
            position: Some(Position { start: 0, end: 0 }),
        },
        // Multiply move should have increased the highest cell
        // reached, but not the current cell. This instruction
        // should not affect the output:
        PointerIncrement {
            amount: 2,
            position: Some(Position { start: 1, end: 1 }),
        },
    ];

    assert_eq!(highest_cell_index(&instrs), 4);
}

/// Multiply move uses offsets to the current pointer value.
/// Verify we add to the current pointer value.
#[test]
fn multiply_move_bounds_are_relative() {
    let mut dest_cells = HashMap::new();
    dest_cells.insert(1, Wrapping(5));
    let instrs = vec![
        // Move to cell #2.
        PointerIncrement {
            amount: 2,
            position: Some(Position { start: 0, end: 0 }),
        },
        // Move (with multiply) to cell #3 (#2 offset 1).
        MultiplyMove {
            changes: dest_cells,
            position: Some(Position { start: 0, end: 0 }),
        },
    ];

    assert_eq!(highest_cell_index(&instrs), 3);
}

#[test]
fn multiply_move_backwards_bounds() {
    let mut dest_cells = HashMap::new();
    dest_cells.insert(-1, Wrapping(2));
    let instrs = vec![
        PointerIncrement {
            amount: 1,
            position: Some(Position { start: 0, end: 0 }),
        },
        MultiplyMove {
            changes: dest_cells,
            position: Some(Position { start: 0, end: 0 }),
        },
    ];

    assert_eq!(highest_cell_index(&instrs), 1);
}

#[test]
fn unbounded_movement() {
    let instrs = parse("[>]").unwrap();
    assert_eq!(highest_cell_index(&instrs), MAX_CELL_INDEX);

    let instrs = parse(">[<]").unwrap();
    assert_eq!(highest_cell_index(&instrs), 1);
}

#[test]
fn excessive_bounds_truncated() {
    // TODO: we should generate a warning in this situation.
    let instrs = vec![PointerIncrement {
        amount: MAX_CELL_INDEX as isize + 1,
        position: Some(Position { start: 0, end: 0 }),
    }];
    assert_eq!(highest_cell_index(&instrs), MAX_CELL_INDEX);
}

#[test]
fn loop_with_no_net_movement() {
    // Max cell index 1, final cell position 0.
    let instrs = parse("[->+<]").unwrap();
    assert_eq!(highest_cell_index(&instrs), 1);

    // Max cell index 1, final cell position 1.
    let instrs = parse("[->+<]>").unwrap();
    assert_eq!(highest_cell_index(&instrs), 1);

    // Max cell index 2, final cell position 2.
    let instrs = parse("[->+<]>>").unwrap();
    assert_eq!(highest_cell_index(&instrs), 2);
}

#[test]
fn quickcheck_highest_cell_index_in_bounds() {
    fn highest_cell_index_in_bounds(instrs: Vec<AstNode>) -> bool {
        let index = highest_cell_index(&instrs);
        index <= MAX_CELL_INDEX
    }
    quickcheck(highest_cell_index_in_bounds as fn(Vec<AstNode>) -> bool);
}

#[test]
fn increment_offset_bounds() {
    let instrs = [Increment {
        amount: Wrapping(2),
        offset: 5,
        position: Some(Position { start: 0, end: 0 }),
    }];
    assert_eq!(highest_cell_index(&instrs), 5);
}

#[test]
fn set_offset_bounds() {
    let instrs = [
        Set {
            amount: Wrapping(2),
            offset: 10,
            position: Some(Position { start: 0, end: 0 }),
        },
        Set {
            amount: Wrapping(2),
            offset: 11,
            position: Some(Position { start: 0, end: 0 }),
        },
    ];
    assert_eq!(highest_cell_index(&instrs), 11);
}
