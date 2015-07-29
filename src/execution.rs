use bfir::parse;
use bfir::Instruction;
use bfir::Instruction::*;

use bounds::highest_cell_index;

#[derive(Debug,PartialEq,Eq)]
struct ExecutionState {
    next: u64,
    // Not all 30,000 cells, just those whose value we know.  Arguably
    // this should be a u8, but it's more convenient to work with (in
    // BF values can wrap around anyway).
    known_cells: Vec<i8>,
    cell_ptr: usize,
    outputs: Vec<i8>
}

#[derive(Debug,PartialEq,Eq)]
enum ExecutionResult {
    Done(ExecutionState),
    ReachedRuntimeValue(ExecutionState),
    OutOfSteps(ExecutionState),
}

// TODO: this is probably not enough.
const MAX_STEPS: u64 = 1000;

/// Compile time speculative execution of instructions. We return the
/// final state of the cells, any print side effects, and the point in
/// the code we reached.
///
/// If we reach an apparently infinite loop, return None.
fn execute(instrs: &Vec<Instruction>, steps: u64) -> ExecutionResult {
    let mut steps = steps;
    
    let cells = vec![0; (highest_cell_index(instrs) + 1) as usize];
    let mut state = ExecutionState {
        next: 0, known_cells: cells, cell_ptr: 0, outputs: vec![] };

    for instr in instrs {
        match instr {
            &Increment(amount) => {
                // TODO: Increment should use an i8.
                state.known_cells[state.cell_ptr] += amount as i8;
            }
            &PointerIncrement(amount) => {
                // TODO: PointerIncrement should use a usize.
                state.cell_ptr += amount as usize;
                // TODO: append more cells as necessary.
            }
            &Write => {
                let cell_value = state.known_cells[state.cell_ptr];
                state.outputs.push(cell_value);
            }
            &Read => {
                return ExecutionResult::ReachedRuntimeValue(state);
            }
            _ => {}
        }
        state.next += 1;
        steps -= 1;

        if steps == 0 {
            return ExecutionResult::OutOfSteps(state);
        }
    }

    ExecutionResult::Done(state)
}

/// We can't evaluate outputs of runtime values at compile time.
#[test]
fn cant_evaluate_inputs() {
    let instrs = parse(",.").unwrap();
    let result = execute(&instrs, MAX_STEPS);

    assert_eq!(
        result,
        ExecutionResult::ReachedRuntimeValue(ExecutionState {
            next: 0, known_cells: vec![0], cell_ptr: 0, outputs: vec![]
        }))
}

#[test]
fn increment_executed() {
    let instrs = parse("+").unwrap();
    let result = execute(&instrs, MAX_STEPS);

    assert_eq!(
        result,
        ExecutionResult::Done(ExecutionState {
            next: 1, known_cells: vec![1], cell_ptr: 0, outputs: vec![]
        }))
}

#[test]
fn decrement_executed() {
    let instrs = parse("+-").unwrap();
    let result = execute(&instrs, MAX_STEPS);

    assert_eq!(
        result,
        ExecutionResult::Done(ExecutionState {
            next: 2, known_cells: vec![0], cell_ptr: 0, outputs: vec![]
        }))
}

#[test]
fn ptr_increment_executed() {
    let instrs = parse(">").unwrap();
    let result = execute(&instrs, MAX_STEPS);

    assert_eq!(
        result,
        ExecutionResult::Done(ExecutionState {
            next: 1, known_cells: vec![0, 0], cell_ptr: 1, outputs: vec![]
        }))
}

#[test]
fn limit_to_steps_specified() {
    let instrs = parse("++++").unwrap();
    let result = execute(&instrs, 2);
    
    assert_eq!(
        result,
        ExecutionResult::OutOfSteps(ExecutionState {
            next: 2, known_cells: vec![2], cell_ptr: 0, outputs: vec![]
        }))
}

#[test]
fn write_executed() {
    let instrs = parse("+.").unwrap();
    let result = execute(&instrs, MAX_STEPS);

    assert_eq!(
        result,
        ExecutionResult::Done(ExecutionState {
            next: 2, known_cells: vec![1], cell_ptr: 0, outputs: vec![1]
        }))
}
