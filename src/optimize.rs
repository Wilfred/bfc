use itertools::Itertools;

use bfir::Instruction;

pub fn optimize(instrs: Vec<Instruction>) -> Vec<Instruction> {
    let combined = combine_ptr_increments(combine_increments(instrs));
    let annotated = annotate_known_zero(combined);
    // Removing dead loops can require us to collapse increments first:
    // Set 1, Increment -1, Loop => Set 0
    // however, it can also create opportunities to collapse increments:
    // Set 0, Loop, Increment 1 => Set 1
    // so we need to run it twice.
    let simplified = combine_set_and_increments(
        remove_dead_loops(combine_set_and_increments(simplify_loops(annotated))));
    remove_pure_code(combine_before_read(remove_redundant_sets(simplified)))
}

/// Combine consecutive increments into a single increment
/// instruction.
pub fn combine_increments(instrs: Vec<Instruction>) -> Vec<Instruction> {
    instrs.into_iter().coalesce(|prev_instr, instr| {
        // Collapse consecutive increments.
        if let (Instruction::Increment(prev_amount), Instruction::Increment(amount)) = (prev_instr.clone(), instr.clone()) {
            Ok(Instruction::Increment(amount + prev_amount))
        } else {
            Err((prev_instr, instr))
        }
    }).filter(|instr| {
        // Remove any increments of 0.
        if let &Instruction::Increment(0) = instr {
            return false;
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

pub fn combine_ptr_increments(instrs: Vec<Instruction>) -> Vec<Instruction> {
    instrs.into_iter().coalesce(|prev_instr, instr| {
        // Collapse consecutive increments.
        if let (Instruction::PointerIncrement(prev_amount), Instruction::PointerIncrement(amount)) = (prev_instr.clone(), instr.clone()) {
            Ok(Instruction::PointerIncrement(amount + prev_amount))
        } else {
            Err((prev_instr, instr))
        }
    }).filter(|instr| {
        // Remove any increments of 0.
        if let &Instruction::PointerIncrement(0) = instr {
            return false;
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

fn combine_before_read(instrs: Vec<Instruction>) -> Vec<Instruction> {
    instrs.into_iter().coalesce(|prev_instr, instr| {
        // Remove dead code before a read.
        match (prev_instr.clone(), instr.clone()) {
            (Instruction::Increment(_), Instruction::Read) => {
                Ok(Instruction::Read)
            },
            (Instruction::Set(_), Instruction::Read) => {
                Ok(Instruction::Read)
            },
            _ => {
                Err((prev_instr, instr))
            }
        }
    }).map(|instr| {
        // Do the same in nested loops.
        match instr {
            Instruction::Loop(body) => {
                Instruction::Loop(combine_before_read(body))
            },
            i => i
        }
    }).collect()
}

pub fn simplify_loops(instrs: Vec<Instruction>) -> Vec<Instruction> {
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

/// Remove any loops where we know the current cell is zero.
pub fn remove_dead_loops(instrs: Vec<Instruction>) -> Vec<Instruction> {
    instrs.into_iter().coalesce(|prev_instr, instr| {
        if let (Instruction::Set(0), Instruction::Loop(_)) = (prev_instr.clone(), instr.clone()) {
            return Ok(Instruction::Set(0));
        }
        Err((prev_instr, instr))
    }).map(|instr| {
        match instr {
            Instruction::Loop(body) => {
                Instruction::Loop(remove_dead_loops(body))
            },
            i => i
        }
    }).collect()
}

/// Combine set instructions with other set instructions or
/// increments.
pub fn combine_set_and_increments(instrs: Vec<Instruction>) -> Vec<Instruction> {
    instrs.into_iter().coalesce(|prev_instr, instr| {
        if let (Instruction::Increment(_), Instruction::Set(amount)) = (prev_instr.clone(), instr.clone()) {
            return Ok(Instruction::Set(amount));
        }
        Err((prev_instr, instr))
    }).coalesce(|prev_instr, instr| {
        if let (Instruction::Set(set_amount), Instruction::Increment(inc_amount)) = (prev_instr.clone(), instr.clone()) {
            return Ok(Instruction::Set(set_amount + inc_amount));
        }
        Err((prev_instr, instr))
    }).coalesce(|prev_instr, instr| {
        if let (Instruction::Set(_), Instruction::Set(amount)) = (prev_instr.clone(), instr.clone()) {
            return Ok(Instruction::Set(amount));
        }
        Err((prev_instr, instr))
    }).map(|instr| {
        match instr {
            Instruction::Loop(body) => {
                Instruction::Loop(combine_set_and_increments(body))
            },
            i => i
        }
    }).collect()
}

pub fn remove_redundant_sets(instrs: Vec<Instruction>) -> Vec<Instruction> {
    let mut reduced: Vec<_> = instrs.into_iter().coalesce(|prev_instr, instr| {
        if let (Instruction::Loop(body), Instruction::Set(0)) = (prev_instr.clone(), instr.clone()) {
            return Ok(Instruction::Loop(body));
        }
        Err((prev_instr, instr))
    }).map(|instr| {
        match instr {
            Instruction::Loop(body) => {
                Instruction::Loop(remove_redundant_sets(body))
            },
            i => i
        }
    }).collect();

    if let Some(&Instruction::Set(0)) = reduced.first() {
        reduced.remove(0);
    }

    reduced
}

pub fn annotate_known_zero(instrs: Vec<Instruction>) -> Vec<Instruction> {
    let mut result = vec![];

    // Cells in BF are initialised to zero, so we know the current
    // cell is zero at the start of execution.
    result.push(Instruction::Set(0));

    result.extend(annotate_known_zero_inner(instrs));
    result
}

fn annotate_known_zero_inner(instrs: Vec<Instruction>) -> Vec<Instruction> {
    let mut result = vec![];

    for instr in instrs {
        match instr {
            // After a loop, we know the cell is currently zero.
            Instruction::Loop(body) => {
                result.push(Instruction::Loop(annotate_known_zero_inner(body)));
                result.push(Instruction::Set(0))
            },
            i => {
                result.push(i);
            }
        }
    }
    result
}

/// Remove code at the end of the program that has no side
/// effects. This means we have no write commands afterwards, nor
/// loops (which may not terminate so we should not remove).
fn remove_pure_code(instrs: Vec<Instruction>) -> Vec<Instruction> {
    let mut seen_side_effect = false;
    let truncated: Vec<Instruction> = instrs.into_iter().rev().skip_while(|instr| {
        match instr {
            &Instruction::Write => {
                seen_side_effect = true;
            },
            &Instruction::Read => {
                seen_side_effect = true;
            },
            &Instruction::Loop(_) => {
                seen_side_effect = true;
            }
            _ => {}
        }
        !seen_side_effect
    }).collect();

    truncated.into_iter().rev().collect()
}
