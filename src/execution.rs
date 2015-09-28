#![warn(trivial_numeric_casts)]

#[cfg(test)]
use std::collections::HashMap;
use std::num::Wrapping;

#[cfg(test)]
use quickcheck::quickcheck;

#[cfg(test)]
use bfir::parse;

use bfir::{Instruction, Cell};
use bfir::Instruction::*;

#[cfg(test)]
use bounds::MAX_CELL_INDEX;

use bounds::highest_cell_index;

// Represents a position within a program. The first index represents
// our position at the top level. If the vector is longer, we're
// mid-way through a loop and each additional index is for a nested
// loop.
//
// For example, given the program +[-]+
// [0] means we haven't executed anything.
// [2] means we've reached the end.
// [1, 0] means we're in the loop, before the -
pub type InstrPosition = Vec<usize>;

#[derive(Debug,Clone,PartialEq,Eq)]
pub struct ExecutionState {
    pub instrs_pos: InstrPosition,
    pub cells: Vec<Cell>,
    pub cell_ptr: isize,
    pub outputs: Vec<i8>,
}

impl ExecutionState {
    fn last_pos(&self) -> usize {
        *self.instrs_pos.last().unwrap()
    }

    fn last_pos_mut(&mut self) -> &mut usize {
        self.instrs_pos.last_mut().unwrap()
    }
}

#[derive(Debug,PartialEq,Eq)]
enum Outcome {
    // Return the number of steps remaining at completion.
    Completed(u64),
    ReachedRuntimeValue,
    RuntimeError,
    OutOfSteps,
}

// It takes around 1 million steps to finish executing bottles.bf at
// compile time. This is intolerably slow for debug builds of bfc, but
// instant on a release build.
pub const MAX_STEPS: u64 = 10000000;

/// Compile time speculative execution of instructions. We return the
/// final state of the cells, any print side effects, and the point in
/// the code we reached.
pub fn execute(instrs: &[Instruction], steps: u64) -> ExecutionState {
    let cells = vec![Wrapping(0); highest_cell_index(instrs) + 1];
    let state = ExecutionState { instrs_pos: vec![0], cells: cells, cell_ptr: 0, outputs: vec![] };
    let (final_state, _) = execute_inner(instrs, state, steps);
    final_state
}

fn execute_inner(instrs: &[Instruction],
                 state: ExecutionState,
                 steps: u64)
                 -> (ExecutionState, Outcome) {
    let mut steps_left = steps;
    let mut state = state;

    while state.last_pos() < instrs.len() && steps_left > 0 {
        let cell_ptr = state.cell_ptr as usize;
        match instrs[state.last_pos()] {
            Increment { amount, offset } => {
                let target_cell_ptr = (cell_ptr as isize + offset) as usize;
                state.cells[target_cell_ptr] = state.cells[target_cell_ptr] + amount;
                *state.last_pos_mut() += 1;
            }
            Set { amount, offset } => {
                let target_cell_ptr = (cell_ptr as isize + offset) as usize;
                state.cells[target_cell_ptr] = amount;
                *state.last_pos_mut() += 1;
            }
            PointerIncrement(amount) => {
                let new_cell_ptr = state.cell_ptr + amount;
                if new_cell_ptr < 0 || new_cell_ptr >= state.cells.len() as isize {
                    return (state, Outcome::RuntimeError);
                } else {
                    state.cell_ptr = new_cell_ptr;
                    *state.last_pos_mut() += 1;
                }
            }
            MultiplyMove(ref changes) => {
                // We will multiply by the current cell value.
                let cell_value = state.cells[cell_ptr];

                for (cell_offset, factor) in changes {
                    let dest_ptr = cell_ptr as isize + *cell_offset;
                    if dest_ptr < 0 {
                        // Tried to access a cell before cell #0.
                        return (state, Outcome::RuntimeError);
                    }
                    if dest_ptr as usize >= state.cells.len() {
                        return (state, Outcome::RuntimeError);
                    }

                    let current_val = state.cells[dest_ptr as usize];
                    state.cells[dest_ptr as usize] = current_val + cell_value * (*factor);
                }

                // Finally, zero the cell we used.
                state.cells[cell_ptr] = Wrapping(0);

                *state.last_pos_mut() += 1;
            }
            Write => {
                let cell_value = state.cells[state.cell_ptr as usize];
                state.outputs.push(cell_value.0);
                *state.last_pos_mut() += 1;
            }
            Read => {
                return (state, Outcome::ReachedRuntimeValue);
            }
            Loop(ref body) => {
                if state.cells[state.cell_ptr as usize].0 == 0 {
                    // Step over the loop because the current cell is
                    // zero.
                    *state.last_pos_mut() += 1;
                } else {
                    // Execute the loop body.
                    let mut instrs_pos = state.instrs_pos.clone();
                    instrs_pos.push(0);
                    let loop_body_state = ExecutionState { instrs_pos: instrs_pos, ..state.clone() };
                    let (state_after, loop_outcome) = execute_inner(body,
                                                                    loop_body_state,
                                                                    steps_left);
                    match loop_outcome {
                        Outcome::Completed(remaining_steps) => {
                            // We finished executing a loop iteration, so store its side effects.
                            state.cells = state_after.cells;
                            state.outputs = state_after.outputs;
                            state.cell_ptr = state_after.cell_ptr;
                            // We've run several steps during the loop
                            // body, so ensure steps_left reflects
                            // that.
                            steps_left = remaining_steps;
                        }
                        Outcome::ReachedRuntimeValue |
                        Outcome::RuntimeError |
                        Outcome::OutOfSteps => {
                            // We couldn't evaluate all of the loop body.
                            return (state_after, loop_outcome);
                        }
                    }
                }
            }
        }

        steps_left -= 1;
    }

    if steps_left == 0 {
        (state, Outcome::OutOfSteps)
    } else {
        (state, Outcome::Completed(steps_left))
    }
}

/// We can't evaluate outputs of runtime values at compile time.
#[test]
fn cant_evaluate_inputs() {
    let instrs = parse(",.").unwrap();
    let final_state = execute(&instrs, MAX_STEPS);

    assert_eq!(final_state,
               ExecutionState {
                   instrs_pos: vec![0],
                   cells: vec![Wrapping(0)],
                   cell_ptr: 0,
                   outputs: vec![],
               });
}

#[test]
fn increment_executed() {
    let instrs = parse("+").unwrap();
    let final_state = execute(&instrs, MAX_STEPS);

    assert_eq!(final_state,
               ExecutionState {
                   instrs_pos: vec![1],
                   cells: vec![Wrapping(1)],
                   cell_ptr: 0,
                   outputs: vec![],
               });
}

#[test]
fn multiply_move_executed() {
    let mut changes = HashMap::new();
    changes.insert(1, Wrapping(2));
    changes.insert(3, Wrapping(3));
    let instrs = vec![// Initial cells: [2, 1, 0, 0]
                      Increment { amount: Wrapping(2), offset: 0 },
                      PointerIncrement(1),
                      Increment { amount: Wrapping(1), offset: 0 },
                      PointerIncrement(-1),

                      MultiplyMove(changes)];

    let final_state = execute(&instrs, MAX_STEPS);
    assert_eq!(final_state,
               ExecutionState {
                   instrs_pos: vec![5],
                   cells: vec![Wrapping(0), Wrapping(5), Wrapping(0), Wrapping(6)],
                   cell_ptr: 0,
                   outputs: vec![],
               });
}

#[test]
fn multiply_move_wrapping() {
    let mut changes = HashMap::new();
    changes.insert(1, Wrapping(3));
    let instrs = vec![Increment { amount: Wrapping(100), offset: 0 }, MultiplyMove(changes)];

    let final_state = execute(&instrs, MAX_STEPS);
    assert_eq!(final_state,
               ExecutionState {
                   instrs_pos: vec![2],
                   // 100 * 3 mod 256 == 44
                   cells: vec![Wrapping(0), Wrapping(44)],
                   cell_ptr: 0,
                   outputs: vec![],
               });
}

#[test]
fn multiply_move_offset_too_high() {
    let mut changes: HashMap<isize, Cell> = HashMap::new();
    changes.insert(MAX_CELL_INDEX as isize + 1, Wrapping(1));
    let instrs = vec![MultiplyMove(changes)];

    let final_state = execute(&instrs, MAX_STEPS);
    assert_eq!(final_state,
               ExecutionState {
                   instrs_pos: vec![0],
                   cells: vec![Wrapping(0); MAX_CELL_INDEX + 1],
                   cell_ptr: 0,
                   outputs: vec![],
               });
}

#[test]
fn multiply_move_offset_too_low() {
    let mut changes = HashMap::new();
    changes.insert(-1, Wrapping(1));
    let instrs = vec![MultiplyMove(changes)];

    let final_state = execute(&instrs, MAX_STEPS);
    assert_eq!(final_state,
               ExecutionState {
                   instrs_pos: vec![0],
                   cells: vec![Wrapping(0)],
                   cell_ptr: 0,
                   outputs: vec![],
               });
}

#[test]
fn set_executed() {
    let instrs = vec![Set { amount: Wrapping(2), offset: 0 }];
    let final_state = execute(&instrs, MAX_STEPS);

    assert_eq!(final_state,
               ExecutionState {
                   instrs_pos: vec![1],
                   cells: vec![Wrapping(2)],
                   cell_ptr: 0,
                   outputs: vec![],
               });
}

#[test]
fn set_wraps() {
    let instrs = vec![Set { amount: Wrapping(-1), offset: 0 }];
    let final_state = execute(&instrs, MAX_STEPS);

    assert_eq!(final_state,
               ExecutionState {
                   instrs_pos: vec![1],
                   cells: vec![Wrapping(-1)],
                   cell_ptr: 0,
                   outputs: vec![],
               });
}

#[test]
fn decrement_executed() {
    let instrs = parse("-").unwrap();
    let final_state = execute(&instrs, MAX_STEPS);

    assert_eq!(final_state,
               ExecutionState {
                   instrs_pos: vec![1],
                   cells: vec![Wrapping(-1)],
                   cell_ptr: 0,
                   outputs: vec![],
               });
}

#[test]
fn increment_wraps() {
    let instrs = vec![Increment { amount: Wrapping(-1), offset: 0 },
                      Increment { amount: Wrapping(1), offset: 0 }];
    let final_state = execute(&instrs, MAX_STEPS);

    assert_eq!(final_state,
               ExecutionState {
                   instrs_pos: vec![2],
                   cells: vec![Wrapping(0)],
                   cell_ptr: 0,
                   outputs: vec![],
               });
}

#[test]
fn ptr_increment_executed() {
    let instrs = parse(">").unwrap();
    let final_state = execute(&instrs, MAX_STEPS);

    assert_eq!(final_state,
               ExecutionState {
                   instrs_pos: vec![1],
                   cells: vec![Wrapping(0), Wrapping(0)],
                   cell_ptr: 1,
                   outputs: vec![],
               });
}

// TODO: it would be nice to emit a warning in this case, as it's
// clearly a user error.
#[test]
fn ptr_out_of_range() {
    let instrs = parse("<").unwrap();
    let final_state = execute(&instrs, MAX_STEPS);

    assert_eq!(final_state,
               ExecutionState {
                   instrs_pos: vec![0],
                   cells: vec![Wrapping(0)],
                   cell_ptr: 0,
                   outputs: vec![],
               });
}

#[test]
fn limit_to_steps_specified() {
    let instrs = parse("++++").unwrap();
    let final_state = execute(&instrs, 2);

    assert_eq!(final_state,
               ExecutionState {
                   instrs_pos: vec![2],
                   cells: vec![Wrapping(2)],
                   cell_ptr: 0,
                   outputs: vec![],
               });
}

#[test]
fn write_executed() {
    let instrs = parse("+.").unwrap();
    let final_state = execute(&instrs, MAX_STEPS);

    assert_eq!(final_state,
               ExecutionState {
                   instrs_pos: vec![2],
                   cells: vec![Wrapping(1)],
                   cell_ptr: 0,
                   outputs: vec![1],
               });
}

#[test]
fn loop_executed() {
    let instrs = parse("++[-]").unwrap();
    let final_state = execute(&instrs, MAX_STEPS);

    assert_eq!(final_state,
               ExecutionState {
                   instrs_pos: vec![3],
                   cells: vec![Wrapping(0)],
                   cell_ptr: 0,
                   outputs: vec![],
               });
}

// If we can't execute all of a loop body, we should still return a
// position within the loop.
#[test]
fn partially_execute_up_to_runtime_value() {
    let instrs = parse("+[[,]]").unwrap();
    let final_state = execute(&instrs, 10);

    assert_eq!(final_state,
               ExecutionState {
                   instrs_pos: vec![1, 0, 0],
                   cells: vec![Wrapping(1)],
                   cell_ptr: 0,
                   outputs: vec![],
               });
}

/// Ensure that we have the correct InstrPosition when we finish
/// executing a top-level loop.
#[test]
fn partially_execute_complete_toplevel_loop() {
    let instrs = parse("+[-],").unwrap();
    let final_state = execute(&instrs, 10);

    assert_eq!(final_state,
               ExecutionState {
                   instrs_pos: vec![2],
                   cells: vec![Wrapping(0)],
                   cell_ptr: 0,
                   outputs: vec![],
               });
}

#[test]
#[test]
fn loop_up_to_step_limit() {
    let instrs = parse("++[-]").unwrap();
    // Assuming we take one step to enter the loop, we will execute
    // the loop body once.
    let final_state = execute(&instrs, 4);

    assert_eq!(final_state,
               ExecutionState {
                   instrs_pos: vec![2],
                   cells: vec![Wrapping(1)],
                   cell_ptr: 0,
                   outputs: vec![],
               });
}

#[test]
fn loop_with_read_body() {
    // We should return the state before the loop is executed, since
    // we can't execute the whole loop.
    let instrs = parse("+[+,]").unwrap();
    let final_state = execute(&instrs, 4);

    assert_eq!(final_state,
               ExecutionState {
                   instrs_pos: vec![1],
                   cells: vec![Wrapping(1)],
                   cell_ptr: 0,
                   outputs: vec![],
               });
}

#[test]
fn up_to_infinite_loop_executed() {
    let instrs = parse("++[]").unwrap();
    let final_state = execute(&instrs, 20);

    assert_eq!(final_state,
               ExecutionState {
                   instrs_pos: vec![2],
                   cells: vec![Wrapping(2)],
                   cell_ptr: 0,
                   outputs: vec![],
               });
}

#[test]
fn quickcheck_instr_ptr_in_bounds() {
    fn instr_ptr_in_bounds(instrs: Vec<Instruction>) -> bool {
        let state = execute(&instrs, 100);
        // TODO: verify all indexes are in bounds.
        state.instrs_pos[0] <= instrs.len()
    }
    quickcheck(instr_ptr_in_bounds as fn(Vec<Instruction>) -> bool);
}

#[test]
fn quickcheck_cell_ptr_in_bounds() {
    fn cell_ptr_in_bounds(instrs: Vec<Instruction>) -> bool {
        let state = execute(&instrs, 100);
        (state.cell_ptr >= 0) && (state.cell_ptr <= state.cells.len() as isize)
    }
    quickcheck(cell_ptr_in_bounds as fn(Vec<Instruction>) -> bool);
}

#[test]
fn arithmetic_error_nested_loops() {
    // Regression test, based on a snippet from
    // mandlebrot.bf. Previously, if the first element in a loop was
    // another loop, we had arithmetic overflow.
    let instrs = parse("+[[>>>>>>>>>]+>>>>>>>>>-]").unwrap();
    execute(&instrs, MAX_STEPS);
}
