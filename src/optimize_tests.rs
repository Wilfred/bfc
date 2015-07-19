use bfir::Instruction;

use optimize::*;
use bfir::parse;
use rand::Rng;
use quickcheck::{Arbitrary,Gen,TestResult};

impl Arbitrary for Instruction {
    fn arbitrary<G: Gen>(g: &mut G) -> Instruction {
        let i = g.next_u32();
        match i % 6 {
            0 => Instruction::Increment(
                Arbitrary::arbitrary(g)),
            1 => Instruction::PointerIncrement(
                Arbitrary::arbitrary(g)),
            2 => Instruction::Set(
                Arbitrary::arbitrary(g)),
            3 => Instruction::Read,
            4 => Instruction::Write,
            5 => Instruction::Loop(vec![]),
            _ => unreachable!()
        }
    }
}

#[test]
fn combine_increments_flat() {
    let initial = parse("++").unwrap();
    let expected = vec![Instruction::Increment(2)];
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
    let expected = vec![Instruction::Loop(vec![
        Instruction::Increment(2)])];
    assert_eq!(combine_increments(initial), expected);
}

#[test]
fn combine_increments_remove_redundant() {
    let initial = parse("+-").unwrap();
    assert_eq!(combine_increments(initial), vec![]);
}

#[test]
fn combine_ptr_increments_flat() {
    let initial = parse(">>").unwrap();
    let expected = vec![Instruction::PointerIncrement(2)];
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
    let expected = vec![Instruction::Loop(vec![
        Instruction::PointerIncrement(2)])];
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
    let expected = vec![
        Instruction::Read,
        Instruction::Write];
    assert_eq!(optimize(initial), expected);
}

#[test]
fn should_combine_before_read_nested() {
    let initial = parse("+[+,]").unwrap();
    let expected = vec![
        Instruction::Set(1),
        Instruction::Loop(vec![
            Instruction::Read])];
    assert_eq!(optimize(initial), expected);
}

#[test]
fn simplify_zeroing_loop() {
    let initial = parse("[-]").unwrap();
    let expected = vec![Instruction::Set(0)];
    assert_eq!(simplify_loops(initial), expected);
}

#[test]
fn simplify_nested_zeroing_loop() {
    let initial = parse("[[-]]").unwrap();
    let expected = vec![Instruction::Loop(vec![Instruction::Set(0)])];
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
        Instruction::Set(0),
        Instruction::Loop(vec![]),
        Instruction::Loop(vec![])];
    let expected = vec![Instruction::Set(0)];
    assert_eq!(remove_dead_loops(initial), expected);
}

#[test]
fn should_remove_dead_loops_nested() {
    let initial = vec![
        Instruction::Loop(vec![
            Instruction::Set(0),
            Instruction::Loop(vec![])])];
    let expected = vec![
        Instruction::Loop(vec![
            Instruction::Set(0)])];
    assert_eq!(remove_dead_loops(initial), expected);
}

#[test]
fn should_combine_set_and_increment() {
    let initial = vec![
        Instruction::Set(0),
        Instruction::Increment(1)];
    let expected = vec![Instruction::Set(1)];
    assert_eq!(combine_set_and_increments(initial), expected);
}

#[test]
fn should_combine_set_and_set() {
    let initial = vec![
        Instruction::Set(0),
        Instruction::Set(1)];
    let expected = vec![Instruction::Set(1)];
    assert_eq!(combine_set_and_increments(initial), expected);
}

#[test]
fn should_combine_set_and_set_nested() {
    let initial = vec![
        Instruction::Loop(vec![
            Instruction::Set(0),
            Instruction::Set(1)])];
    let expected = vec![
        Instruction::Loop(vec![
            Instruction::Set(1)])];
    assert_eq!(combine_set_and_increments(initial), expected);
}

#[test]
fn should_combine_increment_and_set() {
    let initial = vec![
        Instruction::Increment(2),
        Instruction::Set(3)];
    let expected = vec![Instruction::Set(3)];
    assert_eq!(combine_set_and_increments(initial), expected);
}

#[test]
fn should_remove_redundant_set() {
    let initial = vec![
        Instruction::Loop(vec![]),
        Instruction::Set(0)];
    let expected = vec![
        Instruction::Loop(vec![])];
    assert_eq!(remove_redundant_sets(initial), expected);
}

fn is_pure(instrs: &Vec<Instruction>) -> bool {
    for instr in instrs {
        match instr {
            &Instruction::Loop(_) => {
                return false;
            },
            &Instruction::Read => {
                return false;
            },
            &Instruction::Write => {
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
        Instruction::Set(0) => TestResult::from_bool(true),
        _ => TestResult::from_bool(false)
    }
}

#[test]
fn should_annotate_known_zero() {
    let initial = parse("+[]").unwrap();
    let expected = vec![
        Instruction::Set(0),
        Instruction::Increment(1),
        Instruction::Loop(vec![]),
        Instruction::Set(0)];
    assert_eq!(annotate_known_zero(initial), expected);
}

#[test]
fn should_annotate_known_zero_nested() {
    let initial = parse("[[]]").unwrap();
    let expected = vec![
        Instruction::Set(0),
        Instruction::Loop(vec![
            Instruction::Loop(vec![]),
            Instruction::Set(0)]),
        Instruction::Set(0)];
    assert_eq!(annotate_known_zero(initial), expected);
}

#[test]
fn should_remove_pure_code() {
    // The final increment here is side-effect free and can be
    // removed.
    let initial = parse("+.+").unwrap();
    let expected = vec![
        Instruction::Set(1),
        Instruction::Write];
    assert_eq!(optimize(initial), expected);
}

#[quickcheck]
fn should_remove_dead_pure_code(instrs: Vec<Instruction>) -> TestResult {
    if !is_pure(&instrs) {
        return TestResult::discard();
    }
    return TestResult::from_bool(optimize(instrs) == vec![]);
}
