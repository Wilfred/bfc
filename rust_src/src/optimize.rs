use itertools::Itertools;

use bfir::Instruction;

pub fn optimize(instrs: Vec<Instruction>) -> Vec<Instruction> {
    combine_ptr_increments(combine_increments(instrs))
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
    let initial = vec![Instruction::Increment(1),
                       Instruction::Increment(1)];
    let expected = vec![Instruction::Increment(2)];
    assert_eq!(combine_increments(initial), expected);
}

#[test]
fn combine_increments_unrelated() {
    let initial = vec![Instruction::Increment(1),
                       Instruction::PointerIncrement(1),
                       Instruction::Increment(1),
                       Instruction::Write];
    let expected = initial.clone();
    assert_eq!(combine_increments(initial), expected);
}

#[test]
fn combine_increments_nested() {
    let initial = vec![Instruction::Loop(vec![
        Instruction::Increment(1),
        Instruction::Increment(1)])];
    let expected = vec![Instruction::Loop(vec![
        Instruction::Increment(2)])];
    assert_eq!(combine_increments(initial), expected);
}

#[test]
fn combine_increments_remove_redundant() {
    let initial = vec![Instruction::Increment(-1),
                       Instruction::Increment(1)];
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
    let initial = vec![Instruction::PointerIncrement(1),
                       Instruction::PointerIncrement(1)];
    let expected = vec![Instruction::PointerIncrement(2)];
    assert_eq!(combine_ptr_increments(initial), expected);
}

#[test]
fn combine_ptr_increments_unrelated() {
    let initial = vec![Instruction::PointerIncrement(1),
                       Instruction::Increment(1),
                       Instruction::PointerIncrement(1),
                       Instruction::Write];
    let expected = initial.clone();
    assert_eq!(combine_ptr_increments(initial), expected);
}

#[test]
fn combine_ptr_increments_nested() {
    let initial = vec![Instruction::Loop(vec![
        Instruction::PointerIncrement(1),
        Instruction::PointerIncrement(1)])];
    let expected = vec![Instruction::Loop(vec![
        Instruction::PointerIncrement(2)])];
    assert_eq!(combine_ptr_increments(initial), expected);
}

#[test]
fn combine_ptr_increments_remove_redundant() {
    let initial = vec![Instruction::PointerIncrement(-1),
                       Instruction::PointerIncrement(1)];
    assert_eq!(combine_ptr_increments(initial), vec![]);
}
