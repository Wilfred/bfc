use bfir::parse;
use bfir::Instruction;
use bfir::Instruction::*;

use bounds::highest_cell_index;

#[derive(Debug,PartialEq,Eq)]
struct ExecutionState {
    instr_ptr: usize,
    // Not all 30,000 cells, just those whose value we know.  Arguably
    // this should be a u8, but it's more convenient to work with (in
    // BF values can wrap around anyway).
    cells: Vec<i8>,
    cell_ptr: usize,
    outputs: Vec<i8>
}

#[derive(Debug,PartialEq,Eq)]
enum Outcome {
    Completed,
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
    let mut steps = steps;
    let mut state = state;

    loop {
        match &instrs[state.instr_ptr] {
            &Increment(amount) => {
                // TODO: Increment should use an i8.
                state.cells[state.cell_ptr] += amount as i8;
            }
            &PointerIncrement(amount) => {
                // TODO: PointerIncrement should use a usize.
                state.cell_ptr += amount as usize;
                // TODO: append more cells as necessary.
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
                    let loop_body_state = ExecutionState { instr_ptr: 0, .. state };
                    let (state_after, loop_outcome) = execute_inner(body, loop_body_state, steps);
                    if let &Outcome::Completed = &loop_outcome {
                        // We finished executing the loop, so carry on.
                        state = state_after;
                    }
                    else {
                        return (state_after, loop_outcome);
                    }
                }
            }
            // TODO: when we're done, we shouldn't need a placeholder
            // at the end.
            _ => unreachable!()
        }
        state.instr_ptr += 1;
        steps -= 1;

        if steps == 0 {
            return (state, Outcome::OutOfSteps);
        }
        if state.instr_ptr == instrs.len() {
            return (state, Outcome::Completed);
        }
    }
}

/// We can't evaluate outputs of runtime values at compile time.
#[test]
fn cant_evaluate_inputs() {
    let instrs = parse(",.").unwrap();
    let final_state = execute(&instrs, MAX_STEPS);

    assert_eq!(
        final_state, ExecutionState {
            instr_ptr: 0, cells: vec![0], cell_ptr: 0, outputs: vec![]
        });
}

#[test]
fn increment_executed() {
    let instrs = parse("+").unwrap();
    let final_state = execute(&instrs, MAX_STEPS);

    assert_eq!(
        final_state, ExecutionState {
            instr_ptr: 1, cells: vec![1], cell_ptr: 0, outputs: vec![]
        });
}

#[test]
fn decrement_executed() {
    let instrs = parse("+-").unwrap();
    let final_state = execute(&instrs, MAX_STEPS);

    assert_eq!(
        final_state, ExecutionState {
            instr_ptr: 2, cells: vec![0], cell_ptr: 0, outputs: vec![]
        });
}

#[test]
fn ptr_increment_executed() {
    let instrs = parse(">").unwrap();
    let final_state = execute(&instrs, MAX_STEPS);

    assert_eq!(
        final_state, ExecutionState {
            instr_ptr: 1, cells: vec![0, 0], cell_ptr: 1, outputs: vec![]
        });
}

#[test]
fn limit_to_steps_specified() {
    let instrs = parse("++++").unwrap();
    let final_state = execute(&instrs, 2);

    assert_eq!(
        final_state, ExecutionState {
            instr_ptr: 2, cells: vec![2], cell_ptr: 0, outputs: vec![]
        });
}

#[test]
fn write_executed() {
    let instrs = parse("+.").unwrap();
    let final_state = execute(&instrs, MAX_STEPS);

    assert_eq!(
        final_state, ExecutionState {
            instr_ptr: 2, cells: vec![1], cell_ptr: 0, outputs: vec![1]
        });
}

#[test]
fn loop_executed() {
    let instrs = parse("++[-]").unwrap();
    let final_state = execute(&instrs, MAX_STEPS);

    assert_eq!(
        final_state, ExecutionState {
            instr_ptr: 3, cells: vec![0], cell_ptr: 0, outputs: vec![]
        });
}
