use std::collections::HashMap;
use std::num::Wrapping;

use bfir::Instruction;
use bfir::Instruction::*;

use optimize::*;
use bfir::parse;
use rand::Rng;
use quickcheck::{Arbitrary,Gen,TestResult};

// TODO: MultiplyMove here.
impl Arbitrary for Instruction {
    fn arbitrary<G: Gen>(g: &mut G) -> Instruction {
        let i = g.next_u32();
        match i % 11 {
            0 => Increment(Wrapping(Arbitrary::arbitrary(g))),
            1 => PointerIncrement(Arbitrary::arbitrary(g)),
            2 => Set(Wrapping(Arbitrary::arbitrary(g))),
            3 => Read,
            4 => Write,
            // TODO: we should be able to generate arbitrary nested
            // instructions, instead of limited range. See
            // https://github.com/BurntSushi/quickcheck/issues/23
            5 => Loop(vec![]),
            6 => Loop(vec![Increment(Wrapping(Arbitrary::arbitrary(g)))]),
            7 => Loop(vec![PointerIncrement(Arbitrary::arbitrary(g))]),
            8 => Loop(vec![Set(Wrapping(Arbitrary::arbitrary(g)))]),
            9 => Loop(vec![Read]),
            10 => Loop(vec![Read]),
            _ => unreachable!()
        }
    }
}

#[test]
fn combine_increments_flat() {
    let initial = parse("++").unwrap();
    let expected = vec![Increment(Wrapping(2))];
    assert_eq!(combine_increments(initial), expected);
}

#[test]
fn combine_increments_unrelated() {
    let initial = parse("+>+.").unwrap();
    let expected = initial.clone();
    assert_eq!(combine_increments(initial), expected);
}

#[test]
fn combine_increments_nested() {
    let initial = parse("[++]").unwrap();
    let expected = vec![Loop(vec![Increment(Wrapping(2))])];
    assert_eq!(combine_increments(initial), expected);
}

#[test]
fn combine_increments_remove_redundant() {
    let initial = parse("+-").unwrap();
    assert_eq!(combine_increments(initial), vec![]);
}

#[test]
fn combine_increment_sum_to_zero() {
    let initial = vec![Increment(Wrapping(255)), Increment(Wrapping(1))];
    assert_eq!(combine_increments(initial), vec![]);
}

#[test]
fn combine_set_sum_to_zero() {
    let initial = vec![Set(Wrapping(255)), Increment(Wrapping(1))];
    assert_eq!(combine_set_and_increments(initial), vec![Set(Wrapping(0))]);
}

#[test]
fn combine_ptr_increments_flat() {
    let initial = parse(">>").unwrap();
    let expected = vec![PointerIncrement(2)];
    assert_eq!(combine_ptr_increments(initial), expected);
}

#[test]
fn combine_ptr_increments_unrelated() {
    let initial = parse(">+>.").unwrap();
    let expected = initial.clone();
    assert_eq!(combine_ptr_increments(initial), expected);
}

#[test]
fn combine_ptr_increments_nested() {
    let initial = parse("[>>]").unwrap();
    let expected = vec![Loop(vec![
        PointerIncrement(2)])];
    assert_eq!(combine_ptr_increments(initial), expected);
}

#[test]
fn combine_ptr_increments_remove_redundant() {
    let initial = parse("><").unwrap();
    assert_eq!(combine_ptr_increments(initial), vec![]);
}

#[test]
fn should_combine_before_read() {
    // The increment before the read is dead and can be removed.
    let initial = parse("+,.").unwrap();
    let expected = vec![Read, Write];
    assert_eq!(optimize(initial), expected);
}

#[test]
fn should_combine_before_read_nested() {
    let initial = parse("+[+,]").unwrap();
    let expected = vec![Set(Wrapping(1)), Loop(vec![Read])];
    assert_eq!(optimize(initial), expected);
}

#[test]
fn simplify_zeroing_loop() {
    let initial = parse("[-]").unwrap();
    let expected = vec![Set(Wrapping(0))];
    assert_eq!(simplify_loops(initial), expected);
}

#[test]
fn simplify_nested_zeroing_loop() {
    let initial = parse("[[-]]").unwrap();
    let expected = vec![Loop(vec![Set(Wrapping(0))])];
    assert_eq!(simplify_loops(initial), expected);
}

#[test]
fn dont_simplify_multiple_decrement_loop() {
    // A user who wrote this probably meant '[-]'. However, if the
    // current cell has the value 3, we would actually wrap around
    // (although BF does not specify this).
    let initial = parse("[--]").unwrap();
    assert_eq!(simplify_loops(initial.clone()), initial);
}

#[test]
fn should_remove_dead_loops() {
    let initial = vec![
        Set(Wrapping(0)),
        Loop(vec![]),
        Loop(vec![])];
    let expected = vec![Set(Wrapping(0))];
    assert_eq!(remove_dead_loops(initial), expected);
}

#[test]
fn should_remove_dead_loops_nested() {
    let initial = vec![Loop(vec![Set(Wrapping(0)),Loop(vec![])])];
    let expected = vec![Loop(vec![Set(Wrapping(0))])];
    assert_eq!(remove_dead_loops(initial), expected);
}

#[quickcheck]
fn should_combine_set_and_increment(set_amount: u8, increment_amount: u8)
                                    -> bool {
    let set_amount = Wrapping(set_amount);
    let increment_amount = Wrapping(increment_amount);
    let initial = vec![
        Set(set_amount),
        Increment(increment_amount)];
    let expected = vec![Set(set_amount + increment_amount)];
    return combine_set_and_increments(initial) == expected;
}

#[quickcheck]
fn should_combine_set_and_set(set_amount_before: u8, set_amount_after: u8)
                              -> bool {
    let initial = vec![
        Set(Wrapping(set_amount_before)),
        Set(Wrapping(set_amount_after))];
    let expected = vec![Set(Wrapping(set_amount_after))];
    return combine_set_and_increments(initial) == expected;
}

#[test]
fn should_combine_set_and_set_nested() {
    let initial = vec![Loop(vec![Set(Wrapping(0)), Set(Wrapping(1))])];
    let expected = vec![Loop(vec![Set(Wrapping(1))])];
    assert_eq!(combine_set_and_increments(initial), expected);
}

#[test]
fn should_combine_increment_and_set() {
    let initial = vec![Increment(Wrapping(2)), Set(Wrapping(3))];
    let expected = vec![Set(Wrapping(3))];
    assert_eq!(combine_set_and_increments(initial), expected);
}

#[test]
fn should_remove_redundant_set() {
    let initial = vec![Loop(vec![]), Set(Wrapping(0))];
    let expected = vec![Loop(vec![])];
    assert_eq!(remove_redundant_sets(initial), expected);
}

fn is_pure(instrs: &[Instruction]) -> bool {
    for instr in instrs {
        match instr {
            &Loop(_) => {
                return false;
            },
            &Read => {
                return false;
            },
            &Write => {
                return false;
            },
            _ => ()
        }
    }
    true
}

#[quickcheck]
fn should_annotate_known_zero_at_start(instrs: Vec<Instruction>) -> TestResult {
    let annotated = annotate_known_zero(instrs);

    match annotated[0] {
        Set(Wrapping(0)) => TestResult::from_bool(true),
        _ => TestResult::from_bool(false)
    }
}

#[test]
fn should_annotate_known_zero() {
    let initial = parse("+[]").unwrap();
    let expected = vec![
        Set(Wrapping(0)),
        Increment(Wrapping(1)),
        Loop(vec![]),
        Set(Wrapping(0))];
    assert_eq!(annotate_known_zero(initial), expected);
}

#[test]
fn should_annotate_known_zero_nested() {
    let initial = parse("[[]]").unwrap();
    let expected = vec![
        Set(Wrapping(0)),
        Loop(vec![
            Loop(vec![]),
            Set(Wrapping(0))]),
        Set(Wrapping(0))];
    assert_eq!(annotate_known_zero(initial), expected);
}

/// When we annotate known zeroes, we have new opportunities for
/// combining instructions and loop removal. However, we should later
/// remove the Set 0 if we haven't combined it.
#[test]
fn should_annotate_known_zero_cleaned_up() {
    let initial = vec![Write];
    assert_eq!(optimize(initial.clone()), initial);
}

#[test]
fn should_preserve_set_0_in_loop() {
    // Regression test.
    let initial = vec![Read, Loop(vec![Set(Wrapping(0))])];
    assert_eq!(optimize(initial.clone()), initial);
}

#[test]
fn should_remove_pure_code() {
    // The final increment here is side-effect free and can be
    // removed.
    let initial = parse("+.+").unwrap();
    let expected = vec![
        Set(Wrapping(1)),
        Write];
    assert_eq!(optimize(initial), expected);
}

#[quickcheck]
fn should_remove_dead_pure_code(instrs: Vec<Instruction>) -> TestResult {
    if !is_pure(&instrs) {
        return TestResult::discard();
    }
    return TestResult::from_bool(optimize(instrs) == vec![]);
}

#[quickcheck]
fn optimize_should_be_idempotent(instrs: Vec<Instruction>) -> bool {
    // Once we've optimized once, running again shouldn't reduce the
    // instructions further. If it does, we're probably running our
    // optimisations in the wrong order.
    let minimal = optimize(instrs.clone());
    return optimize(minimal.clone()) == minimal;
}

#[test]
fn pathological_optimisation_opportunity() {
    let instrs = vec![Read,
                      Increment(Wrapping(1)),
                      PointerIncrement(1),
                      Increment(Wrapping(1)),
                      PointerIncrement(1),
                      PointerIncrement(-1),
                      Increment(Wrapping(255)),
                      PointerIncrement(-1),
                      Increment(Wrapping(255)),
                      Write];

    let expected = vec![Read, Write];

    assert_eq!(optimize(instrs), expected);
}

fn count_instrs(instrs: &[Instruction]) -> u64 {
    let mut count = 0;
    for instr in instrs {
        if let &Loop(ref body) = instr {
            count += count_instrs(body);
        }
        count += 1;
    }
    count
}

#[quickcheck]
fn optimize_should_decrease_size(instrs: Vec<Instruction>) -> bool {
    // The result of optimize() should never increase the number of
    // instructions.
    let result = optimize(instrs.clone());
    return count_instrs(&result) <= count_instrs(&instrs);
}

#[test]
fn should_extract_multiply_simple() {
    let instrs = parse("[->+++<]").unwrap();

    let mut dest_cells = HashMap::new();
    dest_cells.insert(1, Wrapping(3));
    let expected = vec![MultiplyMove(dest_cells)];

    assert_eq!(extract_multiply(instrs), expected);
}

#[test]
fn should_extract_multiply_nested() {
    let instrs = parse("[[->+<]]").unwrap();

    let mut dest_cells = HashMap::new();
    dest_cells.insert(1, Wrapping(1));
    let expected = vec![
        Loop(vec![
            MultiplyMove(dest_cells)])];

    assert_eq!(extract_multiply(instrs), expected);
}

#[test]
fn should_extract_multiply_negative_number() {
    let instrs = parse("[->--<]").unwrap();

    let mut dest_cells = HashMap::new();
    dest_cells.insert(1, Wrapping(254));
    let expected = vec![MultiplyMove(dest_cells)];

    assert_eq!(extract_multiply(instrs), expected);
}

#[test]
fn should_extract_multiply_multiple_cells() {
    let instrs = parse("[->+++>>>+<<<<]").unwrap();

    let mut dest_cells = HashMap::new();
    dest_cells.insert(1, Wrapping(3));
    dest_cells.insert(4, Wrapping(1));
    let expected = vec![MultiplyMove(dest_cells)];

    assert_eq!(extract_multiply(instrs), expected);
}

#[test]
fn should_not_extract_multiply_net_movement() {
    let instrs = parse("[->+++<<]").unwrap();
    assert_eq!(extract_multiply(instrs.clone()), instrs);
}

#[test]
fn should_not_extract_multiply_from_clear_loop() {
    let instrs = parse("[-]").unwrap();
    assert_eq!(extract_multiply(instrs.clone()), instrs);
}

#[test]
fn should_not_extract_multiply_with_inner_loop() {
    let instrs = parse("[->+++<[]]").unwrap();
    assert_eq!(extract_multiply(instrs.clone()), instrs);
}

/// We need to decrement the initial cell in order for this to be a
/// multiply.
#[test]
fn should_not_extract_multiply_without_decrement() {
    let instrs = parse("[+>++<]").unwrap();
    assert_eq!(extract_multiply(instrs.clone()), instrs);
}

