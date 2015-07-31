use bfir::parse;
use bfir::Instruction;
use bfir::Instruction::*;

use bounds::highest_cell_index;

#[derive(Debug,Clone,PartialEq,Eq)]
struct ExecutionState {
    instr_ptr: usize,
    cells: Vec<u8>,
    cell_ptr: usize,
    outputs: Vec<u8>,
}

#[derive(Debug,PartialEq,Eq)]
enum Outcome {
    // Return the number of steps remaining at completion.
    Completed(u64),
    ReachedRuntimeValue,
    OutOfSteps,
}

// TODO: this is probably not enough.
const MAX_STEPS: u64 = 1000;


/// Compile time speculative execution of instructions. We return the
/// final state of the cells, any print side effects, and the point in
/// the code we reached.
fn execute(instrs: &Vec<Instruction>, steps: u64) -> ExecutionState {
    let cells = vec![0; (highest_cell_index(instrs) + 1) as usize];
    let state = ExecutionState {
        instr_ptr: 0, cells: cells, cell_ptr: 0, outputs: vec![] };
    let (final_state, _) = execute_inner(instrs, state, steps);
    final_state
}

fn execute_inner(instrs: &Vec<Instruction>, state: ExecutionState, steps: u64)
                 -> (ExecutionState, Outcome) {
    let mut steps_left = steps;
    let mut state = state;

    while state.instr_ptr < instrs.len() && steps_left > 0 {
        match &instrs[state.instr_ptr] {
            &Increment(amount) => {
                // TODO: Increment should use an i8.
                state.cells[state.cell_ptr] = state.cells[state.cell_ptr].wrapping_add(amount as u8);
            }
            &Set(amount) => {
                // TODO: Set should use an i8.
                state.cells[state.cell_ptr] = amount as u8;
            }
            &PointerIncrement(amount) => {
                // TODO: PointerIncrement should use a usize.
                state.cell_ptr += amount as usize;
            }
            &Write => {
                let cell_value = state.cells[state.cell_ptr];
                state.outputs.push(cell_value);
            }
            &Read => {
                return (state, Outcome::ReachedRuntimeValue);
            }
            &Loop(ref body) => {
                if state.cells[state.cell_ptr] != 0 {
                    let loop_body_state = ExecutionState { instr_ptr: 0, .. state.clone() };
                    let (state_after, loop_outcome) = execute_inner(body, loop_body_state, steps_left);
                    if let &Outcome::Completed(remaining_steps) = &loop_outcome {
                        // We finished executing a loop iteration, so store its side effects.
                        state.cells = state_after.cells;
                        state.outputs = state_after.outputs;
                        state.cell_ptr = state_after.cell_ptr;
                        // We've run several steps during the loop body, so update for that too.
                        steps_left = remaining_steps;
                        // Go back to the start of the loop.
                        state.instr_ptr -= 1;
                    }
                    else {
                        return (state, loop_outcome);
                    }
                }
                // Otherwise, we step over the loop because the
                // current cell is zero.
            }
        }

        state.instr_ptr += 1;
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

    assert_eq!(
        final_state, ExecutionState {
            instr_ptr: 0, cells: vec![0], cell_ptr: 0, outputs: vec![],
        });
}

#[test]
fn increment_executed() {
    let instrs = parse("+").unwrap();
    let final_state = execute(&instrs, MAX_STEPS);

    assert_eq!(
        final_state, ExecutionState {
            instr_ptr: 1, cells: vec![1], cell_ptr: 0, outputs: vec![],
        });
}

#[test]
fn set_executed() {
    let instrs = vec![Set(2)];
    let final_state = execute(&instrs, MAX_STEPS);

    assert_eq!(
        final_state, ExecutionState {
            instr_ptr: 1, cells: vec![2], cell_ptr: 0, outputs: vec![],
        });
}

#[test]
fn set_wraps() {
    // TODO: this won't be an accurate test when we move to u8.
    let instrs = vec![Set(-1)];
    let final_state = execute(&instrs, MAX_STEPS);

    assert_eq!(
        final_state, ExecutionState {
            instr_ptr: 1, cells: vec![255], cell_ptr: 0, outputs: vec![],
        });
}

#[test]
fn decrement_executed() {
    let instrs = parse("-").unwrap();
    let final_state = execute(&instrs, MAX_STEPS);

    assert_eq!(
        final_state, ExecutionState {
            instr_ptr: 1, cells: vec![255], cell_ptr: 0, outputs: vec![],
        });
}

// TODO: find out what the most common BF implementation choice is here.
#[test]
fn increment_wraps() {
    // TODO: this won't be an accurate test when we move to u8.
    let instrs = vec![Increment(255), Increment(1)];
    let final_state = execute(&instrs, MAX_STEPS);

    assert_eq!(
        final_state, ExecutionState {
            instr_ptr: 2, cells: vec![0], cell_ptr: 0, outputs: vec![],
        });
}

#[test]
fn ptr_increment_executed() {
    let instrs = parse(">").unwrap();
    let final_state = execute(&instrs, MAX_STEPS);

    assert_eq!(
        final_state, ExecutionState {
            instr_ptr: 1, cells: vec![0, 0], cell_ptr: 1, outputs: vec![],
        });
}

#[test]
fn limit_to_steps_specified() {
    let instrs = parse("++++").unwrap();
    let final_state = execute(&instrs, 2);

    assert_eq!(
        final_state, ExecutionState {
            instr_ptr: 2, cells: vec![2], cell_ptr: 0, outputs: vec![],
        });
}

#[test]
fn write_executed() {
    let instrs = parse("+.").unwrap();
    let final_state = execute(&instrs, MAX_STEPS);

    assert_eq!(
        final_state, ExecutionState {
            instr_ptr: 2, cells: vec![1], cell_ptr: 0, outputs: vec![1],
        });
}

#[test]
fn loop_executed() {
    let instrs = parse("++[-]").unwrap();
    let final_state = execute(&instrs, MAX_STEPS);

    assert_eq!(
        final_state, ExecutionState {
            instr_ptr: 3, cells: vec![0], cell_ptr: 0, outputs: vec![],
        });
}

#[test]
fn loop_up_to_step_limit() {
    let instrs = parse("++[-]").unwrap();
    // Assuming we take one step to enter the loop, we will execute
    // the loop body once.
    let final_state = execute(&instrs, 4);

    assert_eq!(
        final_state, ExecutionState {
            instr_ptr: 2, cells: vec![1], cell_ptr: 0, outputs: vec![],
        });
}

#[test]
fn loop_with_read_body() {
    // We should return the state before the loop is executed, since
    // we can't execute the whole loop.
    let instrs = parse("+[+,]").unwrap();
    let final_state = execute(&instrs, 4);

    assert_eq!(
        final_state, ExecutionState {
            instr_ptr: 1, cells: vec![1], cell_ptr: 0, outputs: vec![],
        });
}

#[test]
fn up_to_infinite_loop_executed() {
    let instrs = parse("++[]").unwrap();
    let final_state = execute(&instrs, 20);

    assert_eq!(
        final_state, ExecutionState {
            instr_ptr: 2, cells: vec![2], cell_ptr: 0, outputs: vec![],
        });
}
