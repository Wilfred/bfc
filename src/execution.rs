#![warn(trivial_numeric_casts)]

//! Compile time execution of BF programs.

#[cfg(test)]
use std::collections::HashMap;
use std::env;
use std::num::Wrapping;

#[cfg(test)]
use pretty_assertions::assert_eq;
#[cfg(test)]
use quickcheck::quickcheck;

#[cfg(test)]
use crate::bfir::{parse, Position};

use crate::bfir::AstNode::*;
use crate::bfir::{AstNode, Cell};

use crate::diagnostics::Warning;

#[cfg(test)]
use crate::bounds::MAX_CELL_INDEX;

use crate::bounds::highest_cell_index;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExecutionState<'a> {
    pub start_instr: Option<&'a AstNode>,
    pub cells: Vec<Cell>,
    pub cell_ptr: isize,
    pub outputs: Vec<i8>,
}

impl<'a> ExecutionState<'a> {
    pub fn initial(instrs: &[AstNode]) -> Self {
        ExecutionState {
            start_instr: None,
            cells: vec![Wrapping(0); highest_cell_index(instrs) + 1],
            cell_ptr: 0,
            outputs: vec![],
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum Outcome {
    // Return the number of steps remaining at completion.
    Completed(u64),
    ReachedRuntimeValue,
    RuntimeError(Warning),
    OutOfSteps,
}

/// The maximum number of steps we should execute at compile time.
pub fn max_steps() -> u64 {
    // It takes around 1 million steps to finish executing bottles.bf
    // at compile time. This is intolerably slow for debug builds of
    // bfc, but instant on a release build.
    let mut steps = 10_000_000;

    if let Some(val) = env::var_os("BFC_MAX_STEPS") {
        steps = val.to_str().unwrap().parse::<u64>().unwrap_or(steps);
    }
    steps
}

/// Compile time speculative execution of instructions. We return the
/// final state of the cells, any print side effects, and the point in
/// the code we reached.
pub fn execute(instrs: &[AstNode], steps: u64) -> (ExecutionState, Option<Warning>) {
    let mut state = ExecutionState::initial(instrs);
    let outcome = execute_with_state(instrs, &mut state, steps, None);

    // Sanity check: if we have a start instruction we
    // can't have executed the entire program at compile time.
    match state.start_instr {
        Some(_) => debug_assert!(!matches!(outcome, Outcome::Completed(_))),
        None => debug_assert!(matches!(outcome, Outcome::Completed(_))),
    }

    match outcome {
        Outcome::RuntimeError(warning) => (state, Some(warning)),
        _ => (state, None),
    }
}

/// Execute the instructions given, updating the state as we go.
/// To avoid infinite loops, stop execution after `steps` steps.
///
/// Execution also stops if we encounter a read instruction.  Users may
/// alternatively pass in a dummy value for the read (used in testing).
pub fn execute_with_state<'a>(
    instrs: &'a [AstNode],
    state: &mut ExecutionState<'a>,
    steps: u64,
    dummy_read_value: Option<i8>,
) -> Outcome {
    let mut steps_left = steps;
    let mut instr_idx = 0;
    while instr_idx < instrs.len() && steps_left > 0 {
        let cell_ptr = state.cell_ptr as usize;

        match instrs[instr_idx] {
            Increment { amount, offset, .. } => {
                let target_cell_ptr = (cell_ptr as isize + offset) as usize;
                state.cells[target_cell_ptr] += amount;
                instr_idx += 1;
            }
            Set { amount, offset, .. } => {
                let target_cell_ptr = (cell_ptr as isize + offset) as usize;
                state.cells[target_cell_ptr] = amount;
                instr_idx += 1;
            }
            PointerIncrement {
                amount, position, ..
            } => {
                let new_cell_ptr = state.cell_ptr + amount;
                if new_cell_ptr < 0 || new_cell_ptr >= state.cells.len() as isize {
                    // We can't execute this instruction, so we'll
                    // execute it at runtime (it'll probably be an
                    // error).
                    state.start_instr = Some(&instrs[instr_idx]);

                    let message = if new_cell_ptr < 0 {
                        format!(
                            "This instruction moves the pointer to cell {}.",
                            new_cell_ptr
                        )
                    } else {
                        format!(
                            "This instruction moves the pointer after the last cell ({}), to \
                             cell {}.",
                            state.cells.len() - 1,
                            new_cell_ptr
                        )
                    };
                    return Outcome::RuntimeError(Warning { message, position });
                } else {
                    state.cell_ptr = new_cell_ptr;
                    instr_idx += 1;
                }
            }
            MultiplyMove {
                ref changes,
                position,
                ..
            } => {
                let cell_value = state.cells[cell_ptr];

                if cell_value.0 != 0 {
                    // We will multiply by the current cell value.

                    for (cell_offset, factor) in changes {
                        let dest_ptr = cell_ptr as isize + *cell_offset;
                        if dest_ptr < 0 {
                            // Tried to access a cell before cell #0.
                            state.start_instr = Some(&instrs[instr_idx]);

                            // TODO: would be nice to have a Hint: message too in compiler warnings.
                            let message = format!(
                                "This multiply loop tried to access cell {} \
                                 (offset {} from current cell {})",
                                dest_ptr, *cell_offset, cell_ptr
                            );

                            return Outcome::RuntimeError(Warning {
                                message,
                                position,
                            });
                        }
                        if dest_ptr as usize >= state.cells.len() {
                            state.start_instr = Some(&instrs[instr_idx]);
                            return Outcome::RuntimeError(Warning {
                                message: format!(
                                    "This multiply loop tried to access cell {} (the \
                                     highest cell is {})",
                                    dest_ptr,
                                    state.cells.len() - 1
                                ),
                                position,
                            });
                        }

                        let current_val = state.cells[dest_ptr as usize];
                        state.cells[dest_ptr as usize] = current_val + cell_value * (*factor);
                    }

                    // Finally, zero the cell we used.
                    state.cells[cell_ptr] = Wrapping(0);
                }

                instr_idx += 1;
            }
            Write { .. } => {
                let cell_value = state.cells[state.cell_ptr as usize];
                state.outputs.push(cell_value.0);
                instr_idx += 1;
            }
            Read { .. } => {
                if let Some(read_value) = dummy_read_value {
                    // If we're given a dummy value to use for the
                    // read, pretend that we've read that value.
                    state.cells[state.cell_ptr as usize] = Wrapping(read_value);
                    instr_idx += 1
                } else {
                    // Otherwise, we cannot proceed at compile time,
                    // so ensure runtime execution starts from here.
                    state.start_instr = Some(&instrs[instr_idx]);
                    return Outcome::ReachedRuntimeValue;
                }
            }
            Loop { ref body, .. } => {
                if state.cells[state.cell_ptr as usize].0 == 0 {
                    // Step over the loop because the current cell is
                    // zero.
                    instr_idx += 1;
                } else {
                    // Execute the loop body.
                    let loop_outcome =
                        execute_with_state(body, state, steps_left, dummy_read_value);
                    match loop_outcome {
                        Outcome::Completed(remaining_steps) => {
                            // We've run several steps during the loop
                            // body, so ensure steps_left reflects
                            // that.
                            steps_left = remaining_steps;
                        }
                        Outcome::ReachedRuntimeValue
                        | Outcome::RuntimeError(..)
                        | Outcome::OutOfSteps => {
                            // If we ran out of steps after a complete
                            // loop iteration, start_instr will still
                            // be None, so we set it to the current loop.
                            if state.start_instr == None {
                                state.start_instr = Some(&instrs[instr_idx]);
                            }
                            return loop_outcome;
                        }
                    }
                }
            }
        }

        steps_left -= 1;
    }

    // If we've run out of steps, runtime execution should start
    // from the next instruction.
    if steps_left == 0 {
        // If the next instruction is in the current loop, use that.
        if instr_idx < instrs.len() {
            state.start_instr = Some(&instrs[instr_idx]);
        }
        // Otherwise, we've run out of steps after executing a
        // complete loop iteration. We'll set the start instruction as
        // the loop.

        Outcome::OutOfSteps
    } else {
        Outcome::Completed(steps_left)
    }
}

/// We can't evaluate outputs of runtime values at compile time.
#[test]
fn cant_evaluate_inputs() {
    let instrs = parse(",.").unwrap();
    let final_state = execute(&instrs, max_steps()).0;

    assert_eq!(
        final_state,
        ExecutionState {
            start_instr: Some(&instrs[0]),
            cells: vec![Wrapping(0)],
            cell_ptr: 0,
            outputs: vec![],
        }
    );
}

#[test]
fn increment_executed() {
    let instrs = parse("+").unwrap();
    let final_state = execute(&instrs, max_steps()).0;

    assert_eq!(
        final_state,
        ExecutionState {
            start_instr: None,
            cells: vec![Wrapping(1)],
            cell_ptr: 0,
            outputs: vec![],
        }
    );
}

#[test]
fn multiply_move_executed() {
    let mut changes = HashMap::new();
    changes.insert(1, Wrapping(2));
    changes.insert(3, Wrapping(3));

    let instrs = [
        // Initial cells: [2, 1, 0, 0]
        Increment {
            amount: Wrapping(2),
            offset: 0,
            position: Some(Position { start: 0, end: 0 }),
        },
        PointerIncrement {
            amount: 1,
            position: Some(Position { start: 0, end: 0 }),
        },
        Increment {
            amount: Wrapping(1),
            offset: 0,
            position: Some(Position { start: 0, end: 0 }),
        },
        PointerIncrement {
            amount: -1,
            position: Some(Position { start: 0, end: 0 }),
        },
        MultiplyMove {
            changes,
            position: Some(Position { start: 0, end: 0 }),
        },
    ];

    let final_state = execute(&instrs, max_steps()).0;
    assert_eq!(
        final_state,
        ExecutionState {
            start_instr: None,
            cells: vec![Wrapping(0), Wrapping(5), Wrapping(0), Wrapping(6)],
            cell_ptr: 0,
            outputs: vec![],
        }
    );
}

/// When the current cell is zero, we shouldn't execute a multiply move instruction.
/// Otherwise, the BF program [-<+>] (which is well formed and does nothing) becomes
/// undefined behaviour when we have a multiply move instruction.
#[test]
fn multiply_move_when_current_cell_is_zero() {
    let mut changes = HashMap::new();
    changes.insert(-1, Wrapping(2));

    let instrs = [MultiplyMove {
        changes,
        position: None,
    }];

    let (final_state, warning) = execute(&instrs, max_steps());
    assert_eq!(warning, None);
    assert_eq!(
        final_state,
        ExecutionState {
            start_instr: None,
            cells: vec![Wrapping(0)],
            cell_ptr: 0,
            outputs: vec![],
        }
    );
}

#[test]
fn multiply_move_wrapping() {
    let mut changes = HashMap::new();
    changes.insert(1, Wrapping(3));
    let instrs = [
        Increment {
            amount: Wrapping(100),
            offset: 0,
            position: Some(Position { start: 0, end: 0 }),
        },
        MultiplyMove {
            changes,
            position: Some(Position { start: 0, end: 0 }),
        },
    ];

    let final_state = execute(&instrs, max_steps()).0;
    assert_eq!(
        final_state,
        ExecutionState {
            start_instr: None,
            // 100 * 3 mod 256 == 44
            cells: vec![Wrapping(0), Wrapping(44)],
            cell_ptr: 0,
            outputs: vec![],
        }
    );
}

#[test]
fn multiply_move_offset_too_high() {
    let mut changes: HashMap<isize, Cell> = HashMap::new();
    changes.insert(MAX_CELL_INDEX as isize + 1, Wrapping(1));
    let instrs = [
        Increment {
            amount: Wrapping(1),
            offset: 0,
            position: None,
        },
        MultiplyMove {
            changes,
            position: Some(Position { start: 0, end: 0 }),
        },
    ];

    let final_state = execute(&instrs, max_steps()).0;
    let mut expected_cells = vec![Wrapping(0); MAX_CELL_INDEX + 1];
    expected_cells[0] = Wrapping(1);
    assert_eq!(
        final_state,
        ExecutionState {
            start_instr: Some(&instrs[1]),
            cells: expected_cells,
            cell_ptr: 0,
            outputs: vec![],
        }
    );
}

#[test]
fn multiply_move_offset_too_low() {
    let mut changes = HashMap::new();
    changes.insert(-1, Wrapping(1));
    let instrs = [
        Increment {
            amount: Wrapping(1),
            offset: 0,
            position: None,
        },
        MultiplyMove {
            changes,
            position: Some(Position { start: 0, end: 0 }),
        },
    ];

    let final_state = execute(&instrs, max_steps()).0;
    assert_eq!(
        final_state,
        ExecutionState {
            start_instr: Some(&instrs[1]),
            cells: vec![Wrapping(1)],
            cell_ptr: 0,
            outputs: vec![],
        }
    );
}

#[test]
fn set_executed() {
    let instrs = [Set {
        amount: Wrapping(2),
        offset: 0,
        position: Some(Position { start: 0, end: 0 }),
    }];
    let final_state = execute(&instrs, max_steps()).0;

    assert_eq!(
        final_state,
        ExecutionState {
            start_instr: None,
            cells: vec![Wrapping(2)],
            cell_ptr: 0,
            outputs: vec![],
        }
    );
}

#[test]
fn set_wraps() {
    let instrs = [Set {
        amount: Wrapping(-1),
        offset: 0,
        position: Some(Position { start: 0, end: 0 }),
    }];
    let final_state = execute(&instrs, max_steps()).0;

    assert_eq!(
        final_state,
        ExecutionState {
            start_instr: None,
            cells: vec![Wrapping(-1)],
            cell_ptr: 0,
            outputs: vec![],
        }
    );
}

#[test]
fn decrement_executed() {
    let instrs = parse("-").unwrap();
    let final_state = execute(&instrs, max_steps()).0;

    assert_eq!(
        final_state,
        ExecutionState {
            start_instr: None,
            cells: vec![Wrapping(-1)],
            cell_ptr: 0,
            outputs: vec![],
        }
    );
}

#[test]
fn increment_wraps() {
    let instrs = [
        Increment {
            amount: Wrapping(-1),
            offset: 0,
            position: Some(Position { start: 0, end: 0 }),
        },
        Increment {
            amount: Wrapping(1),
            offset: 0,
            position: Some(Position { start: 0, end: 0 }),
        },
    ];
    let final_state = execute(&instrs, max_steps()).0;

    assert_eq!(
        final_state,
        ExecutionState {
            start_instr: None,
            cells: vec![Wrapping(0)],
            cell_ptr: 0,
            outputs: vec![],
        }
    );
}

#[test]
fn ptr_increment_executed() {
    let instrs = parse(">").unwrap();
    let final_state = execute(&instrs, max_steps()).0;

    assert_eq!(
        final_state,
        ExecutionState {
            start_instr: None,
            cells: vec![Wrapping(0), Wrapping(0)],
            cell_ptr: 1,
            outputs: vec![],
        }
    );
}

#[test]
fn ptr_out_of_range() {
    let instrs = parse("<").unwrap();
    let (final_state, warning) = execute(&instrs, max_steps());

    assert_eq!(
        final_state,
        ExecutionState {
            start_instr: Some(&instrs[0]),
            cells: vec![Wrapping(0)],
            cell_ptr: 0,
            outputs: vec![],
        }
    );

    assert!(warning.is_some());
}

#[test]
fn limit_to_steps_specified() {
    let instrs = parse("++++").unwrap();
    let final_state = execute(&instrs, 2).0;

    assert_eq!(
        final_state,
        ExecutionState {
            start_instr: Some(&instrs[2]),
            cells: vec![Wrapping(2)],
            cell_ptr: 0,
            outputs: vec![],
        }
    );
}

#[test]
fn write_executed() {
    let instrs = parse("+.").unwrap();
    let final_state = execute(&instrs, max_steps()).0;

    assert_eq!(
        final_state,
        ExecutionState {
            start_instr: None,
            cells: vec![Wrapping(1)],
            cell_ptr: 0,
            outputs: vec![1],
        }
    );
}

#[test]
fn loop_executed() {
    let instrs = parse("++[-]").unwrap();
    let final_state = execute(&instrs, max_steps()).0;

    assert_eq!(
        final_state,
        ExecutionState {
            start_instr: None,
            cells: vec![Wrapping(0)],
            cell_ptr: 0,
            outputs: vec![],
        }
    );
}

// If we can't execute all of a loop body, we should still return a
// position within the loop.
#[test]
fn partially_execute_up_to_runtime_value() {
    let instrs = parse("+[[,]]").unwrap();
    let final_state = execute(&instrs, 10).0;

    // Get the inner read instruction
    let start_instr = match instrs[1] {
        Loop { ref body, .. } => match body[0] {
            Loop {
                body: ref body2, ..
            } => &body2[0],
            _ => unreachable!(),
        },
        _ => unreachable!(),
    };
    assert_eq!(
        *start_instr,
        Read {
            position: Some(Position { start: 3, end: 3 })
        }
    );

    assert_eq!(
        final_state,
        ExecutionState {
            start_instr: Some(start_instr),
            cells: vec![Wrapping(1)],
            cell_ptr: 0,
            outputs: vec![],
        }
    );
}

#[test]
fn execute_read_with_dummy_value() {
    let instrs = parse(",").unwrap();

    let mut state = ExecutionState::initial(&instrs[..]);
    execute_with_state(&instrs[..], &mut state, 5, Some(1));

    assert_eq!(state.cells[0], Wrapping(1));
}

#[test]
fn execute_read_with_dummy_value_nested_loop() {
    // Regression test.
    let instrs = parse("+[[,]]").unwrap();

    let mut state = ExecutionState::initial(&instrs[..]);
    let outcome = execute_with_state(&instrs[..], &mut state, 20, Some(0));

    assert!(matches!(outcome, Outcome::Completed(_)));
}

/// Ensure that we have the correct InstrPosition when we finish
/// executing a top-level loop.
#[test]
fn partially_execute_complete_toplevel_loop() {
    let instrs = parse("+[-],").unwrap();
    let final_state = execute(&instrs, 10).0;

    assert_eq!(
        final_state,
        ExecutionState {
            start_instr: Some(&instrs[2]),
            cells: vec![Wrapping(0)],
            cell_ptr: 0,
            outputs: vec![],
        }
    );
}

#[test]
fn partially_execute_up_to_step_limit() {
    let instrs = parse("+[++++]").unwrap();
    let final_state = execute(&instrs, 3).0;

    let start_instr = match instrs[1] {
        Loop { ref body, .. } => &body[2],
        _ => unreachable!(),
    };

    assert_eq!(
        final_state,
        ExecutionState {
            start_instr: Some(start_instr),
            cells: vec![Wrapping(3)],
            cell_ptr: 0,
            outputs: vec![],
        }
    );
}

#[test]
fn loop_up_to_step_limit() {
    let instrs = parse("++[-]").unwrap();
    // Assuming we take one step to enter the loop, we will execute
    // the loop body once.
    let final_state = execute(&instrs, 4).0;

    assert_eq!(
        final_state,
        ExecutionState {
            start_instr: Some(&instrs[2]),
            cells: vec![Wrapping(1)],
            cell_ptr: 0,
            outputs: vec![],
        }
    );
}

#[test]
fn loop_with_read_body() {
    // We can't execute the whole loop, so our start instruction
    // should be the read.
    let instrs = parse("+[+,]").unwrap();
    let final_state = execute(&instrs, 4).0;

    // Get the inner read instruction
    let start_instr = match instrs[1] {
        Loop { ref body, .. } => &body[1],
        _ => unreachable!(),
    };
    assert_eq!(
        *start_instr,
        Read {
            position: Some(Position { start: 3, end: 3 })
        }
    );

    assert_eq!(
        final_state,
        ExecutionState {
            start_instr: Some(start_instr),
            cells: vec![Wrapping(2)],
            cell_ptr: 0,
            outputs: vec![],
        }
    );
}

#[test]
fn up_to_infinite_loop_executed() {
    let instrs = parse("++[]").unwrap();
    let final_state = execute(&instrs, 20).0;

    assert_eq!(
        final_state,
        ExecutionState {
            start_instr: Some(&instrs[2]),
            cells: vec![Wrapping(2)],
            cell_ptr: 0,
            outputs: vec![],
        }
    );
}

#[test]
fn up_to_nonempty_infinite_loop() {
    let instrs = parse("+[+]").unwrap();
    let final_state = execute(&instrs, 20).0;

    assert_eq!(
        final_state,
        ExecutionState {
            start_instr: Some(&instrs[1]),
            cells: vec![Wrapping(11)],
            cell_ptr: 0,
            outputs: vec![],
        }
    );
}

#[test]
fn quickcheck_cell_ptr_in_bounds() {
    fn cell_ptr_in_bounds(instrs: Vec<AstNode>) -> bool {
        let state = execute(&instrs, 100).0;
        (state.cell_ptr >= 0) && (state.cell_ptr < state.cells.len() as isize)
    }
    quickcheck(cell_ptr_in_bounds as fn(Vec<AstNode>) -> bool);
}

#[test]
fn arithmetic_error_nested_loops() {
    // Regression test, based on a snippet from
    // mandlebrot.bf. Previously, if the first element in a loop was
    // another loop, we had arithmetic overflow.
    let instrs = parse("+[[>>>>>>>>>]+>>>>>>>>>-]").unwrap();
    execute(&instrs, max_steps());
}
