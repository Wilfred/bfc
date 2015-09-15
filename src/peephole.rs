use std::hash::Hash;
use std::collections::HashMap;
use std::num::Wrapping;

use itertools::Itertools;

use bfir::{Instruction, Cell};
use bfir::Instruction::*;

/// Given a sequence of BF instructions, apply peephole optimisations
/// (repeatedly if necessary).
pub fn optimize(instrs: Vec<Instruction>) -> Vec<Instruction> {
    // Many of our individual peephole optimisations remove
    // instructions, creating new opportunities to combine. We run
    // until we've found a fixed-point where no further optimisations
    // can be made.
    let mut prev = instrs.clone();
    let mut result = optimize_once(instrs);
    while prev != result {
        prev = result.clone();
        result = optimize_once(result);
    }
    result
}

/// Apply all our peephole optimisations once and return the result.
fn optimize_once(instrs: Vec<Instruction>) -> Vec<Instruction> {
    let combined = combine_increments(instrs);
    let annotated = annotate_known_zero(combined);
    let extracted = extract_multiply(annotated);
    let simplified = remove_dead_loops(combine_set_and_increments(simplify_loops(extracted)));
    let removed = remove_pure_code(combine_before_read(remove_redundant_sets(simplified)));
    sort_by_offset(removed)
}

/// Defines a method on iterators to map a function over all loop bodies.
trait MapLoopsExt: Iterator<Item=Instruction> {
    fn map_loops<F>(&mut self, f: F) -> Vec<Instruction>
        where F: Fn(Vec<Instruction>) -> Vec<Instruction>
    {
        self.map(|instr| {
            match instr {
                Loop(body) => Loop(f(body)),
                other => other
            }
        }).collect()
    }
}

impl<I> MapLoopsExt for I where I: Iterator<Item=Instruction> { }

/// Given an index into a vector of instructions, find the previous
/// instruction that modified the current cell. If we're unsure, or
/// there isn't one, return None.
///
/// Note this ignores offsets of the instruction at the index. E.g. if
/// that instruction is Set{amount:100, offset: 1}, we're still
/// considering previous instructions that modify the current cell,
/// not the (cell_index + 1)th cell.
pub fn previous_cell_change(instrs: Vec<Instruction>, index: usize) -> Option<Instruction> {
    assert!(index < instrs.len());

    let mut needed_offset = 0;
    for i in (0..index).rev() {
        match instrs[i] {
            Increment { amount, offset} => {
                if offset == needed_offset {
                    return Some(Increment { amount: amount, offset: offset })
                }
            }
            Set { amount, offset} => {
                if offset == needed_offset {
                    return Some(Set { amount: amount, offset: offset })
                }
            }
            PointerIncrement(amount) => {
                needed_offset += amount;
            }
            MultiplyMove(ref changes) => {
                // These cells are written to.
                let mut offsets: Vec<isize> = changes.keys().into_iter().map(|offset| { *offset }).collect();
                // This cell is zeroed.
                offsets.push(0);
                
                if offsets.contains(&needed_offset) {
                    return Some(MultiplyMove(changes.clone()));
                }
            }
            // No cells changed, so just keep working backwards.
            Write => {}
            // These instructions may have modified the cell, so
            // we return None for "I don't know".
            Read | Loop(_) => return None,
        }
    }
    None
}

/// Combine consecutive increments into a single increment
/// instruction.
pub fn combine_increments(instrs: Vec<Instruction>) -> Vec<Instruction> {
    instrs.into_iter().coalesce(|prev_instr, instr| {
        // Collapse consecutive increments.
        if let &Increment { amount: prev_amount, offset: prev_offset } = &prev_instr {
            if let &Increment { amount, offset } = &instr {
                if prev_offset == offset {
                    return Ok(Increment { amount: amount + prev_amount, offset: offset });
                }
            }
        }
        return Err((prev_instr, instr));
    }).filter(|instr| {
        // Remove any increments of 0.
        if let &Increment{ amount: Wrapping(0), .. } = instr {
            return true;
        }
        false
    }).map(|instr| {
        // Combine increments in nested loops too.
        match instr {
            Loop(body) => {
                Loop(combine_increments(body))
            },
            i => i
        }
    }).collect()
}

fn combine_before_read(instrs: Vec<Instruction>) -> Vec<Instruction> {
    instrs.into_iter().coalesce(|prev_instr, instr| {
        // Remove redundant code before a read.
        match (prev_instr, instr) {
            (Increment{..}, Read) => Ok(Read),
            (Set{ offset: 0, .. }, Read) => Ok(Read),
            tuple => Err(tuple)
        }
    }).map_loops(combine_before_read)
}

pub fn simplify_loops(instrs: Vec<Instruction>) -> Vec<Instruction> {
    instrs.into_iter().map(|instr| {
        if let &Loop(ref body) = &instr {
            // If the loop is [-]
            if *body == vec![Increment { amount: Wrapping(-1), offset: 0 }] {
                return Set { amount: Wrapping(0), offset: 0 }
            }
        }
        instr
    }).map_loops(simplify_loops)
}

/// Remove any loops where we know the current cell is zero.
pub fn remove_dead_loops(instrs: Vec<Instruction>) -> Vec<Instruction> {
    // TODO: search back further if we've normalised increments.
    instrs.into_iter().coalesce(|prev_instr, instr| {
        if let (&Set { amount: Wrapping(0), offset: 0 }, &Loop(_)) = (&prev_instr, &instr) {
            return Ok(Set { amount: Wrapping(0), offset: 0 });
        }
        Err((prev_instr, instr))
    }).map_loops(remove_dead_loops)
}

// TODO: document in README
// TODO: update other optimisations now that we can't just
// look at the next/previous instruction.
pub fn sort_by_offset(instrs: Vec<Instruction>) -> Vec<Instruction> {
    let mut sequence = vec![];
    let mut result = vec![];

    for instr in instrs {
        match instr {
            Increment{..} | Set{..} | PointerIncrement(_) => {
                sequence.push(instr);
            }
            _ => {
                if !sequence.is_empty() {
                    result.extend(sort_sequence_by_offset(sequence));
                    sequence = vec![];
                }
                if let Loop(body) = instr {
                    result.push(Loop(sort_by_offset(body)));
                } else {
                    result.push(instr);
                }
            }
        }
    }
    
    if !sequence.is_empty() {
        result.extend(sort_sequence_by_offset(sequence));
    }

    result
}

/// Given a HashMap with orderable keys, return the values according to
/// the key order.
/// {2: 'foo': 1: 'bar'} => vec!['bar', 'foo']
fn ordered_values<K: Ord + Hash + Eq, V>(map: HashMap<K, V>) -> Vec<V> {
    let mut items: Vec<_> = map.into_iter().collect();
    items.sort_by(|a, b| a.0.cmp(&b.0));
    items.into_iter().map(|(_, v)| { v }).collect()
}

/// Given a BF program, combine sets/increments using offsets so we
/// have single PointerIncrement at the end.
pub fn sort_sequence_by_offset(instrs: Vec<Instruction>) -> Vec<Instruction> {
    let mut instrs_by_offset: HashMap<isize,Vec<Instruction>> = HashMap::new();
    let mut current_offset = 0;

    for instr in instrs {
        match instr {
            Increment { amount, offset } => {
                let new_offset = offset + current_offset;
                let same_offset_instrs = instrs_by_offset.entry(new_offset)
                    .or_insert(vec![]);
                same_offset_instrs.push(Increment { amount: amount, offset: new_offset});
            }
            Set { amount, offset } => {
                let new_offset = offset + current_offset;
                let same_offset_instrs = instrs_by_offset.entry(new_offset)
                    .or_insert(vec![]);
                same_offset_instrs.push(Set { amount: amount, offset: new_offset});
            },
            PointerIncrement(amount) => {
                current_offset += amount;
            },
            // We assume that we were only given a Vec of
            // Increment/Set/PointerIncrement instructions. It's
            // the job of this function to create instructions with
            // offset.
            _ => unreachable!()
        }
    }

    let mut results: Vec<Instruction> = vec![];
    for same_offset_instrs in ordered_values(instrs_by_offset) {
        results.extend(same_offset_instrs.into_iter());
    }

    if current_offset != 0 {
        results.push(PointerIncrement(current_offset));
    }
    results
}

/// Combine set instructions with other set instructions or
/// increments.
pub fn combine_set_and_increments(instrs: Vec<Instruction>) -> Vec<Instruction> {
    // TODO: Handle arbitrary offsets, or rewrite as a normalise_increments optimisation.
    instrs.into_iter().coalesce(|prev_instr, instr| {
        if let (&Increment { offset: 0, .. }, &Set { amount, offset: 0 }) = (&prev_instr, &instr) {
            return Ok(Set { amount: amount, offset: 0 });
        }
        Err((prev_instr, instr))
    }).coalesce(|prev_instr, instr| {
        if let (&Set { amount: set_amount, offset: 0 }, &Increment { amount: inc_amount, offset: 0 }) = (&prev_instr, &instr) {
            return Ok(Set { amount: set_amount + inc_amount, offset: 0 });
        }
        Err((prev_instr, instr))
    }).coalesce(|prev_instr, instr| {
        if let (&Set { offset: 0, .. }, &Set { amount, offset: 0 }) = (&prev_instr, &instr) {
            return Ok(Set { amount: amount, offset: 0 });
        }
        Err((prev_instr, instr))
    }).map_loops(combine_set_and_increments)
}

pub fn remove_redundant_sets(instrs: Vec<Instruction>) -> Vec<Instruction> {
    let mut reduced = remove_redundant_sets_inner(instrs);

    if let Some(&Set { amount: Wrapping(0), offset: 0 }) = reduced.first() {
        reduced.remove(0);
    }

    reduced
}

fn remove_redundant_sets_inner(instrs: Vec<Instruction>) -> Vec<Instruction> {
    instrs.into_iter().coalesce(|prev_instr, instr| {
        match (&prev_instr, &instr) {
            (&Loop(_), &Set{ amount: Wrapping(0), offset: 0 }) => Ok(prev_instr),
            (&MultiplyMove(_), &Set{ amount: Wrapping(0), offset: 0}) => Ok(prev_instr),
            _ => Err((prev_instr, instr))
        }
    }).map_loops(remove_redundant_sets_inner)
}

pub fn annotate_known_zero(instrs: Vec<Instruction>) -> Vec<Instruction> {
    let mut result = vec![];

    // Cells in BF are initialised to zero, so we know the current
    // cell is zero at the start of execution.
    result.push(Set { amount: Wrapping(0), offset: 0 });

    result.extend(annotate_known_zero_inner(instrs));
    result
}

fn annotate_known_zero_inner(instrs: Vec<Instruction>) -> Vec<Instruction> {
    let mut result = vec![];

    for instr in instrs {
        match instr {
            // After a loop, we know the cell is currently zero.
            Loop(body) => {
                result.push(Loop(annotate_known_zero_inner(body)));
                result.push(Set { amount: Wrapping(0), offset: 0 })
            }
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
fn remove_pure_code(mut instrs: Vec<Instruction>) -> Vec<Instruction> {
    for index in (0..instrs.len()).rev() {
        match instrs[index] {
            Read | Write | Loop(_) => {
                instrs.truncate(index + 1);
                return instrs;
            }
            _ => {}
        }
    }
    vec![]
}

/// Does this loop body represent a multiplication operation?
/// E.g. "[->>>++<<<]" sets cell #3 to 2*cell #0.
fn is_multiply_loop_body(body: &[Instruction]) -> bool {
    // A multiply loop may only contain increments and pointer increments.
    for body_instr in body {
        match *body_instr {
            Increment{..} => {}
            PointerIncrement(_) => {}
            _ => return false,
        }
    }

    // A multiply loop must have a net pointer movement of
    // zero.
    let mut net_movement = 0;
    for body_instr in body {
        if let PointerIncrement(amount) = *body_instr {
            net_movement += amount;
        }
    }
    if net_movement != 0 {
        return false;
    }

    let changes = cell_changes(body);
    // A multiply loop must decrement cell #0.
    if let Some(&Wrapping(-1)) = changes.get(&0) {
    } else {
        return false;
    }

    changes.len() >= 2
}

/// Return a hashmap of all the cells that are affected by this
/// sequence of instructions, and how much they change.
/// E.g. "->>+++>+" -> {0: -1, 2: 3, 3: 1}
fn cell_changes(instrs: &[Instruction]) -> HashMap<isize, Cell> {
    let mut changes = HashMap::new();
    let mut cell_index: isize = 0;

    for instr in instrs {
        match *instr {
            Increment{ amount, offset } => {
                let current_amount = *changes.get(&(cell_index + offset)).unwrap_or(&Wrapping(0));
                changes.insert(cell_index, current_amount + amount);
            }
            PointerIncrement(amount) => {
                cell_index += amount;
            }
            // We assume this is only called from is_multiply_loop.
            _ => unreachable!(),
        }
    }

    changes
}

pub fn extract_multiply(instrs: Vec<Instruction>) -> Vec<Instruction> {
    instrs.into_iter().map(|instr| {
        match instr {
            Loop(body) => {
                if is_multiply_loop_body(&body) {
                    let mut changes = cell_changes(&body);
                    // MultiplyMove is for where we move to, so ignore
                    // the cell we're moving from.
                    changes.remove(&0);

                    MultiplyMove(changes)
                } else {
                    Loop(extract_multiply(body))
                }
            }
            i => i
        }
    }).collect()
}
