use bfir::Instruction;

// TODO: mark this as unused only when we're not running tests.
#[allow(unused_imports)]
use bfir::parse;

const MAX_CELLS: u64 = 30000;

pub fn highest_cell_index(instrs: Vec<Instruction>) -> u64 {
    MAX_CELLS
}

#[test]
fn unbounded_should_return_max() {
    let instrs = parse("[>]").unwrap();
    assert_eq!(highest_cell_index(instrs), MAX_CELLS);
}
