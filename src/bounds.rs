use bfir::Instruction;

// TODO: mark this as unused only when we're not running tests.
#[allow(unused_imports)]
use bfir::parse;

const MAX_CELLS: i64 = 30000;

pub fn highest_cell_index(instrs: &Vec<Instruction>) -> u64 {
    let mut max_seen: i64 = 1;

    // TODO: smarter handling of loops, smarter handling of pointer
    // decrement.
    for instr in instrs {
        match instr {
            &Instruction::Loop(_) => {
                max_seen = MAX_CELLS;
                break;
            },
            &Instruction::PointerIncrement(amount) => {
                max_seen += amount as i64;
            },
            _ => {}
        }
    }

    max_seen as u64
}

#[test]
fn one_cell_bounds() {
    let instrs = parse("+-.,").unwrap();
    assert_eq!(highest_cell_index(&instrs), 1);
}

#[test]
fn ptr_increment_bounds() {
    let instrs = parse(">").unwrap();
    assert_eq!(highest_cell_index(&instrs), 2);
}

#[test]
fn multiple_ptr_increment_bounds() {
    let instrs = vec![Instruction::PointerIncrement(2)];
    assert_eq!(highest_cell_index(&instrs), 3);
}

#[test]
fn unbounded_should_return_max() {
    let instrs = parse("[>]").unwrap();
    assert_eq!(highest_cell_index(&instrs), MAX_CELLS as u64);
}
