use itertools::Itertools;

use bfir::{Instruction,parse};

pub fn optimize(instrs: Vec<Instruction>) -> Vec<Instruction> {
    let combined = combine_ptr_increments(combine_increments(instrs));
    remove_dead_loops(simplify_loops(combined))
}

/// Combine consecutive increments into a single increment
/// instruction.
fn combine_increments(instrs: Vec<Instruction>) -> Vec<Instruction> {
    instrs.into_iter().coalesce(|prev_instr, instr| {
        // Collapse consecutive increments.
        if let (Instruction::Increment(prev_amount), Instruction::Increment(amount)) = (prev_instr.clone(), instr.clone()) {
            Ok(Instruction::Increment(amount + prev_amount))
        } else {
            Err((prev_instr, instr))
        }
    }).filter(|instr| {
        // Remove any increments of 0.
        if let &Instruction::Increment(amount) = instr {
            println!("amount: {}", amount);
            if amount == 0 {
                return false;
            }
        }
        true
    }).map(|instr| {
        // Combine increments in nested loops too.
        match instr {
            Instruction::Loop(body) => {
                Instruction::Loop(combine_increments(body))
            },
            i => i
        }
    }).collect()
}

#[test]
fn combine_increments_flat() {
    let initial = parse("++");
    let expected = vec![Instruction::Increment(2)];
    assert_eq!(combine_increments(initial), expected);
}

#[test]
fn combine_increments_unrelated() {
    let initial = parse("+>+.");
    let expected = initial.clone();
    assert_eq!(combine_increments(initial), expected);
}

#[test]
fn combine_increments_nested() {
    let initial = parse("+[++]");
    let expected = vec![Instruction::Loop(vec![
        Instruction::Increment(2)])];
    assert_eq!(combine_increments(initial), expected);
}

#[test]
fn combine_increments_remove_redundant() {
    let initial = parse("+-");
    assert_eq!(combine_increments(initial), vec![]);
}

fn combine_ptr_increments(instrs: Vec<Instruction>) -> Vec<Instruction> {
    instrs.into_iter().coalesce(|prev_instr, instr| {
        // Collapse consecutive increments.
        if let (Instruction::PointerIncrement(prev_amount), Instruction::PointerIncrement(amount)) = (prev_instr.clone(), instr.clone()) {
            Ok(Instruction::PointerIncrement(amount + prev_amount))
        } else {
            Err((prev_instr, instr))
        }
    }).filter(|instr| {
        // Remove any increments of 0.
        if let &Instruction::PointerIncrement(amount) = instr {
            if amount == 0 {
                return false;
            }
        }
        true
    }).map(|instr| {
        // Combine increments in nested loops too.
        match instr {
            Instruction::Loop(body) => {
                Instruction::Loop(combine_ptr_increments(body))
            },
            i => i
        }
    }).collect()
}

#[test]
fn combine_ptr_increments_flat() {
    let initial = parse(">>");
    let expected = vec![Instruction::PointerIncrement(2)];
    assert_eq!(combine_ptr_increments(initial), expected);
}

#[test]
fn combine_ptr_increments_unrelated() {
    let initial = parse(">+>.");
    let expected = initial.clone();
    assert_eq!(combine_ptr_increments(initial), expected);
}

#[test]
fn combine_ptr_increments_nested() {
    let initial = parse("[>>]");
    let expected = vec![Instruction::Loop(vec![
        Instruction::PointerIncrement(2)])];
    assert_eq!(combine_ptr_increments(initial), expected);
}

#[test]
fn combine_ptr_increments_remove_redundant() {
    let initial = parse("><");
    assert_eq!(combine_ptr_increments(initial), vec![]);
}

fn simplify_loops(instrs: Vec<Instruction>) -> Vec<Instruction> {
    instrs.into_iter().map(|instr| {
        if let Instruction::Loop(body) = instr.clone() {
            if body == vec![Instruction::Increment(-1)] {
                return Instruction::Set(0)
            }
        }
        instr
    }).map(|instr| {
        // Simplify zeroing loops nested in other loops.
        match instr {
            Instruction::Loop(body) => {
                Instruction::Loop(simplify_loops(body))
            },
            i => i
        }
    }).collect()
}

#[test]
fn simplify_zeroing_loop() {
    let initial = parse("[-]");
    let expected = vec![Instruction::Set(0)];
    assert_eq!(simplify_loops(initial), expected);
}

#[test]
fn simplify_nested_zeroing_loop() {
    let initial = parse("[[-]]");
    let expected = vec![Instruction::Loop(vec![Instruction::Set(0)])];
    assert_eq!(simplify_loops(initial), expected);
}

#[test]
fn dont_simplify_multiple_decrement_loop() {
    // A user who wrote this probably meant '[-]'. However, if the
    // current cell has the value 3, we would actually wrap around
    // (although BF does not specify this).
    let initial = parse("[--]");
    assert_eq!(simplify_loops(initial.clone()), initial);
}

/// Remove any loops where we know the current cell is zero.
fn remove_dead_loops(instrs: Vec<Instruction>) -> Vec<Instruction> {
    // TODO: nested dead loops.
    instrs.into_iter().coalesce(|prev_instr, instr| {
        if let (Instruction::Set(amount), Instruction::Loop(_)) = (prev_instr.clone(), instr.clone()) {
            if amount == 0 {
                return Ok(Instruction::Set(amount));
            }
        }
        Err((prev_instr, instr))
    }).collect()
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

