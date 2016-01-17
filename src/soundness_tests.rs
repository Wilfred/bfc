use std::num::Wrapping;
use quickcheck::{quickcheck, TestResult};

use bfir::Instruction;
use execution::{execute_inner, ExecutionState};
use execution::Outcome::*;
use peephole::*;


fn transform_is_sound<F>(instrs: Vec<Instruction>, transform: F, check_cells: bool) -> TestResult
    where F: Fn(Vec<Instruction>) -> Vec<Instruction>
{
    let max_steps = 1000;
    let max_cells = 1000;

    // First, we execute the program given.
    let mut state = ExecutionState {
        start_instr: None,
        cells: vec![Wrapping(0); max_cells],
        cell_ptr: 0,
        outputs: vec![],
    };
    let result = execute_inner(&instrs[..], &mut state, max_steps);

    // Optimisations may change malformed programs to well-formed
    // programs, so we ignore programs that don't terminate nicely.
    match result {
        RuntimeError(_) | OutOfSteps => return TestResult::discard(),
        _ => (),
    }

    // Next, we execute the program after transformation.
    let optimised_instrs = transform(instrs.clone());
    let mut state2 = ExecutionState {
        start_instr: None,
        cells: vec![Wrapping(0); max_cells],
        cell_ptr: 0,
        outputs: vec![],
    };
    let result2 = execute_inner(&optimised_instrs[..], &mut state2, max_steps);

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
        println!("Different outputs! Optimised program: {:?}",
                 optimised_instrs);
        return TestResult::failed();
    }

    // If requested, compare that the cells at the end are the same
    // too. This is true of most, but not all, of our optimisations.
    if check_cells {
        if state.cells != state2.cells {
            println!("Different cell states! Optimised program: {:?}",
                     optimised_instrs);
            return TestResult::failed();
        }
    }

    TestResult::passed()
}

#[test]
fn combine_increments_is_sound() {
    fn is_sound(instrs: Vec<Instruction>) -> TestResult {
        transform_is_sound(instrs, combine_increments, true)
    }
    quickcheck(is_sound as fn(Vec<Instruction>) -> TestResult)
}

#[test]
fn combine_ptr_increments_is_sound() {
    fn is_sound(instrs: Vec<Instruction>) -> TestResult {
        transform_is_sound(instrs, combine_ptr_increments, true)
    }
    quickcheck(is_sound as fn(Vec<Instruction>) -> TestResult)
}

#[test]
fn annotate_known_zero_is_sound() {
    fn is_sound(instrs: Vec<Instruction>) -> TestResult {
        transform_is_sound(instrs, annotate_known_zero, true)
    }
    quickcheck(is_sound as fn(Vec<Instruction>) -> TestResult)
}

#[test]
fn extract_multiply_is_sound() {
    fn is_sound(instrs: Vec<Instruction>) -> TestResult {
        transform_is_sound(instrs, extract_multiply, true)
    }
    quickcheck(is_sound as fn(Vec<Instruction>) -> TestResult)
}

#[test]
fn simplify_loops_is_sound() {
    fn is_sound(instrs: Vec<Instruction>) -> TestResult {
        transform_is_sound(instrs, simplify_loops, true)
    }
    quickcheck(is_sound as fn(Vec<Instruction>) -> TestResult)
}

#[test]
fn combine_set_and_increments_is_sound() {
    fn is_sound(instrs: Vec<Instruction>) -> TestResult {
        transform_is_sound(instrs, combine_set_and_increments, true)
    }
    quickcheck(is_sound as fn(Vec<Instruction>) -> TestResult)
}

#[test]
fn remove_dead_loops_is_sound() {
    fn is_sound(instrs: Vec<Instruction>) -> TestResult {
        transform_is_sound(instrs, remove_dead_loops, true)
    }
    quickcheck(is_sound as fn(Vec<Instruction>) -> TestResult)
}

#[test]
fn remove_redundant_sets_is_sound() {
    fn is_sound(instrs: Vec<Instruction>) -> TestResult {
        transform_is_sound(instrs, remove_redundant_sets, true)
    }
    quickcheck(is_sound as fn(Vec<Instruction>) -> TestResult)
}

#[test]
fn combine_before_read_is_sound() {
    fn is_sound(instrs: Vec<Instruction>) -> TestResult {
        // combine_before_read can change the value of cells when we
        // reach a runtime value. Conside `+,` to `,` -- the `,`
        // overwrites the cell, but when we reach the runtime value
        // the cells are different.
        transform_is_sound(instrs, combine_before_read, false)
    }
    quickcheck(is_sound as fn(Vec<Instruction>) -> TestResult)
}


#[test]
fn remove_pure_code_is_sound() {
    fn is_sound(instrs: Vec<Instruction>) -> TestResult {
        // We can't compare cells after this pass. Consider `.+` to
        // `.` -- the outputs are the same, but the cell state is
        // different at termination.
        transform_is_sound(instrs, |instrs| remove_pure_code(instrs).0, false)
    }
    quickcheck(is_sound as fn(Vec<Instruction>) -> TestResult)
}

#[test]
fn sort_by_offset_is_sound() {
    fn is_sound(instrs: Vec<Instruction>) -> TestResult {
        transform_is_sound(instrs, sort_by_offset, true)
    }
    quickcheck(is_sound as fn(Vec<Instruction>) -> TestResult)
}

#[test]
fn test_overall_optimize_is_sound() {
    fn optimize_ignore_warnings(instrs: Vec<Instruction>) -> Vec<Instruction> {
        optimize(instrs).0
    }

    fn optimizations_sound_together(instrs: Vec<Instruction>) -> TestResult {
        // Since sort_by_offset and combine_before_read can change
        // cell values at termination, the overall optimize can change
        // cells values at termination.
        transform_is_sound(instrs, optimize_ignore_warnings, false)
    }

    quickcheck(optimizations_sound_together as fn(Vec<Instruction>) -> TestResult);
}
