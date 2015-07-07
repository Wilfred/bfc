use bfir::Instruction;

pub fn combine_increments(instrs: Vec<Instruction>) -> Vec<Instruction> {
    let mut result = vec![];
    let mut previous: Option<Instruction> = None;

    for instr in instrs {
        match previous {
            // If the previous instruction was an increment:
            Some(Instruction::Increment(prev_amount)) => {
                // and the current instruction was an increment:
                if let Instruction::Increment(amount) = instr {
                    // then combine the two instructions.
                    if amount + prev_amount == 0 {
                        previous = None
                    } else {
                        previous = Some(Instruction::Increment(amount + prev_amount));
                    }
                } else {
                    // Otherwise, iterate as normal.
                    result.push(Instruction::Increment(prev_amount));
                    previous = Some(instr);
                }
            },
            Some(prev_instr) => {
                result.push(prev_instr);
                previous = Some(instr);
            }
            // First iteration.
            None => {
                previous = Some(instr);
            }
        }
    }
    if let Some(instr) = previous {
        result.push(instr);
    }

    // Combine increments in nested loops too.
    result.into_iter().map(|instr| {
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

