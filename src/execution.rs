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
    outputs: Vec<u8>
}

// TODO: this is probably not enough.
const MAX_STEPS: u64 = 1000;

/// Compile time speculative execution of instructions. We return the
/// final state of the cells, any print side effects, and the point in
/// the code we reached.
///
/// If we reach an apparently infinite loop, return None.
fn execute(instrs: &Vec<Instruction>, _: u64) -> Option<ExecutionState> {
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
            &Read => { break; }
            &Write => { break; }
            _ => {}
        }
        state.next += 1;
    }

    Some(state)
}

/// We can't evaluate outputs of runtime values at compile time.
#[test]
fn cant_evaluate_inputs() {
    let instrs = parse(",.").unwrap();
    let result = execute(&instrs, MAX_STEPS);

    assert_eq!(
        result,
        Some(ExecutionState { next: 0, known_cells: vec![0], cell_ptr: 0, outputs: vec![] }))
}

#[test]
fn increment_executed() {
    let instrs = parse("+").unwrap();
    let result = execute(&instrs, MAX_STEPS);

    assert_eq!(
        result,
        Some(ExecutionState { next: 1, known_cells: vec![1], cell_ptr: 0, outputs: vec![] }))
}

#[test]
fn decrement_executed() {
    let instrs = parse("+-").unwrap();
    let result = execute(&instrs, MAX_STEPS);

    assert_eq!(
        result,
        Some(ExecutionState { next: 2, known_cells: vec![0], cell_ptr: 0, outputs: vec![] }))
}

#[test]
fn ptr_increment_executed() {
    let instrs = parse(">").unwrap();
    let result = execute(&instrs, MAX_STEPS);

    assert_eq!(
        result,
        Some(ExecutionState { next: 1, known_cells: vec![0, 0], cell_ptr: 1, outputs: vec![] }))
}
