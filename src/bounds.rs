use bfir::Instruction;

// TODO: mark this as unused only when we're not running tests.
#[allow(unused_imports)]
use bfir::parse;

const MAX_CELLS: i64 = 30000;

/// Return the highest cell index that can be reached during program
/// execution. Zero-indexed.
pub fn highest_cell_index(instrs: &Vec<Instruction>) -> u64 {
    let mut max_overall: i64 = 0;
    let mut max_at_point = 0;

    // TODO: smarter handling of loops
    for instr in instrs {
        match instr {
            &Instruction::Loop(_) => {
                // TODO: use saturating arithmetic
                max_at_point = MAX_CELLS;
            },
            &Instruction::PointerIncrement(amount) => {
                max_at_point += amount as i64;
            },
            _ => {}
        }
        if max_at_point > max_overall {
            max_overall = max_at_point;
        }
    }

    max_overall as u64
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
    let instrs = vec![Instruction::PointerIncrement(2)];
    assert_eq!(highest_cell_index(&instrs), 2);
}

#[test]
fn unbounded_should_return_max() {
    let instrs = parse("[>]").unwrap();
    assert_eq!(highest_cell_index(&instrs), MAX_CELLS as u64);
}
