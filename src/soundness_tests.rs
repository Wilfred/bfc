use quickcheck::{quickcheck, TestResult};

use crate::bfir::AstNode;
use crate::execution::Outcome::*;
use crate::execution::{execute_with_state, ExecutionState};
use crate::peephole::*;

fn transform_is_sound<F>(
    instrs: Vec<AstNode>,
    transform: F,
    check_cells: bool,
    dummy_read_value: Option<i8>,
) -> TestResult
where
    F: Fn(Vec<AstNode>) -> Vec<AstNode>,
{
    let max_steps = 1000;

    // First, we execute the program given.
    let mut state = ExecutionState::initial(&instrs[..]);
    let result = execute_with_state(&instrs[..], &mut state, max_steps, dummy_read_value);

    // Optimisations may change malformed programs to well-formed
    // programs, so we ignore programs that don't terminate nicely.
    match result {
        RuntimeError(_) | OutOfSteps => return TestResult::discard(),
        _ => (),
    }

    // Next, we execute the program after transformation.
    let optimised_instrs = transform(instrs.clone());
    // Deliberately start our state from the original instrs, so we
    // get the same number of cells. Otherwise we could get in messy
    // situations where a dead loop that makes us think we use
    // MAX_CELLS so state2 has fewer cells.
    let mut state2 = ExecutionState::initial(&instrs[..]);
    let result2 = execute_with_state(
        &optimised_instrs[..],
        &mut state2,
        max_steps,
        dummy_read_value,
    );

    // Compare the outcomes: they should be the same.
    match (result, result2) {
        // If the first result completed, the second should have
        // completed too. We allow them to take a different amount of
        // steps.
        (Completed(_), Completed(_)) => (),
        (ReachedRuntimeValue, ReachedRuntimeValue) => (),
        // Any other situation means that the first program terminated
        // but the optimised program did not.
        (_, _) => {
            println!("Optimised program did not terminate properly!");
            return TestResult::failed();
        }
    }

    // Likewise we should have written the same outputs.
    if state.outputs != state2.outputs {
        println!(
            "Different outputs! Original outputs: {:?} Optimised: {:?}",
            state.outputs, state2.outputs
        );
        return TestResult::failed();
    }

    // If requested, compare that the cells at the end are the same
    // too. This is true of most, but not all, of our optimisations.
    if check_cells && state.cells != state2.cells {
        println!(
            "Different cell states! Optimised state: {:?} Optimised: {:?}",
            state.cells, state2.cells
        );
        return TestResult::failed();
    }

    TestResult::passed()
}

#[test]
fn combine_increments_is_sound() {
    fn is_sound(instrs: Vec<AstNode>) -> TestResult {
        transform_is_sound(instrs, combine_increments, true, None)
    }
    quickcheck(is_sound as fn(Vec<AstNode>) -> TestResult)
}

#[test]
fn combine_ptr_increments_is_sound() {
    fn is_sound(instrs: Vec<AstNode>) -> TestResult {
        transform_is_sound(instrs, combine_ptr_increments, true, None)
    }
    quickcheck(is_sound as fn(Vec<AstNode>) -> TestResult)
}

#[test]
fn annotate_known_zero_is_sound() {
    fn is_sound(instrs: Vec<AstNode>) -> TestResult {
        transform_is_sound(instrs, annotate_known_zero, true, None)
    }
    quickcheck(is_sound as fn(Vec<AstNode>) -> TestResult)
}

#[test]
fn extract_multiply_is_sound() {
    fn is_sound(instrs: Vec<AstNode>) -> TestResult {
        transform_is_sound(instrs, extract_multiply, true, None)
    }
    quickcheck(is_sound as fn(Vec<AstNode>) -> TestResult)
}

#[test]
fn simplify_loops_is_sound() {
    fn is_sound(instrs: Vec<AstNode>) -> TestResult {
        transform_is_sound(instrs, zeroing_loops, true, None)
    }
    quickcheck(is_sound as fn(Vec<AstNode>) -> TestResult)
}

#[test]
fn combine_set_and_increments_is_sound() {
    fn is_sound(instrs: Vec<AstNode>) -> TestResult {
        transform_is_sound(instrs, combine_set_and_increments, true, None)
    }
    quickcheck(is_sound as fn(Vec<AstNode>) -> TestResult)
}

#[test]
fn remove_dead_loops_is_sound() {
    fn is_sound(instrs: Vec<AstNode>) -> TestResult {
        transform_is_sound(instrs, remove_dead_loops, true, None)
    }
    quickcheck(is_sound as fn(Vec<AstNode>) -> TestResult)
}

#[test]
fn remove_redundant_sets_is_sound() {
    fn is_sound(instrs: Vec<AstNode>) -> TestResult {
        transform_is_sound(instrs, remove_redundant_sets, true, None)
    }
    quickcheck(is_sound as fn(Vec<AstNode>) -> TestResult)
}

#[test]
fn combine_before_read_is_sound() {
    fn is_sound(instrs: Vec<AstNode>, read_value: Option<i8>) -> TestResult {
        // remove_read_clobber can change the value of cells when we
        // reach a runtime value. Conside `+,` to `,` -- the `,`
        // overwrites the cell, but when we reach the runtime value
        // the cells are different.
        transform_is_sound(instrs, remove_read_clobber, false, read_value)
    }
    quickcheck(is_sound as fn(Vec<AstNode>, Option<i8>) -> TestResult)
}

#[test]
fn remove_pure_code_is_sound() {
    fn is_sound(instrs: Vec<AstNode>) -> TestResult {
        // We can't compare cells after this pass. Consider `.+` to
        // `.` -- the outputs are the same, but the cell state is
        // different at termination.
        transform_is_sound(instrs, |instrs| remove_pure_code(instrs).0, false, None)
    }
    quickcheck(is_sound as fn(Vec<AstNode>) -> TestResult)
}

#[test]
fn sort_by_offset_is_sound() {
    fn is_sound(instrs: Vec<AstNode>) -> TestResult {
        transform_is_sound(instrs, sort_by_offset, true, None)
    }
    quickcheck(is_sound as fn(Vec<AstNode>) -> TestResult)
}

#[test]
fn test_overall_optimize_is_sound() {
    fn optimize_ignore_warnings(instrs: Vec<AstNode>) -> Vec<AstNode> {
        optimize(instrs, &None).0
    }

    fn optimizations_sound_together(instrs: Vec<AstNode>, read_value: Option<i8>) -> TestResult {
        // Since sort_by_offset and remove_read_clobber can change
        // cell values at termination, the overall optimize can change
        // cells values at termination.
        transform_is_sound(instrs, optimize_ignore_warnings, false, read_value)
    }

    quickcheck(optimizations_sound_together as fn(Vec<AstNode>, Option<i8>) -> TestResult);
}
