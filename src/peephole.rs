//! Optimisations that replace parts of the BF AST with faster
//! equivalents.

use std::collections::{HashMap, HashSet};
use std::hash::Hash;
use std::num::Wrapping;

use itertools::Itertools;

use crate::diagnostics::Warning;

use crate::bfir::AstNode::*;
use crate::bfir::{get_position, AstNode, BfValue, Combine, Position};

const MAX_OPT_ITERATIONS: u64 = 40;

/// Given a sequence of BF instructions, apply peephole optimisations
/// (repeatedly if necessary).
pub fn optimize(
    instrs: Vec<AstNode>,
    pass_specification: &Option<String>,
) -> (Vec<AstNode>, Vec<Warning>) {
    // Many of our individual peephole optimisations remove
    // instructions, creating new opportunities to combine. We run
    // until we've found a fixed-point where no further optimisations
    // can be made.
    let mut prev = instrs.clone();
    let mut warnings = vec![];

    let (mut result, warning) = optimize_once(instrs, pass_specification);

    if let Some(warning) = warning {
        warnings.push(warning);
    }

    for _ in 0..MAX_OPT_ITERATIONS {
        if prev == result {
            return (result, warnings);
        } else {
            prev = result.clone();

            let (new_result, new_warning) = optimize_once(result, pass_specification);

            if let Some(warning) = new_warning {
                warnings.push(warning);
            }
            result = new_result;
        }
    }

    // TODO: use proper Info here.
    eprintln!(
        "Warning: ran peephole optimisations {} times but did not reach a fixed point!",
        MAX_OPT_ITERATIONS
    );

    (result, warnings)
}

/// Apply all our peephole optimisations once and return the result.
fn optimize_once(
    instrs: Vec<AstNode>,
    pass_specification: &Option<String>,
) -> (Vec<AstNode>, Option<Warning>) {
    let pass_specification = pass_specification.clone().unwrap_or_else(|| {
        "combine_inc,combine_ptr,known_zero,\
         multiply,zeroing_loop,combine_set,\
         dead_loop,redundant_set,read_clobber,\
         pure_removal,offset_sort"
            .to_owned()
    });
    let passes: Vec<_> = pass_specification.split(',').collect();

    let mut instrs = instrs;

    if passes.contains(&"combine_inc") {
        instrs = combine_increments(instrs);
    }
    if passes.contains(&"combine_ptr") {
        instrs = combine_ptr_increments(instrs);
    }
    if passes.contains(&"known_zero") {
        instrs = annotate_known_zero(instrs);
    }
    if passes.contains(&"multiply") {
        instrs = extract_multiply(instrs);
    }
    if passes.contains(&"zeroing_loop") {
        instrs = zeroing_loops(instrs);
    }
    if passes.contains(&"combine_set") {
        instrs = combine_set_and_increments(instrs);
    }
    if passes.contains(&"dead_loop") {
        instrs = remove_dead_loops(instrs);
    }
    if passes.contains(&"redundant_set") {
        instrs = remove_redundant_sets(instrs);
    }
    if passes.contains(&"read_clobber") {
        instrs = remove_read_clobber(instrs);
    }
    let warning = if passes.contains(&"pure_removal") {
        let (removed, pure_warning) = remove_pure_code(instrs);
        instrs = removed;
        pure_warning
    } else {
        None
    };

    if passes.contains(&"offset_sort") {
        instrs = sort_by_offset(instrs);
    }

    (instrs, warning)
}

/// Defines a method on iterators to map a function over all loop bodies.
trait MapLoopsExt: Iterator<Item = AstNode> {
    fn map_loops<F>(&mut self, f: F) -> Vec<AstNode>
    where
        F: Fn(Vec<AstNode>) -> Vec<AstNode>,
    {
        self.map(|instr| match instr {
            Loop { body, position } => Loop {
                body: f(body),
                position,
            },
            other => other,
        })
        .collect()
    }
}

impl<I> MapLoopsExt for I where I: Iterator<Item = AstNode> {}

/// Given an index into a vector of instructions, find the index of
/// the previous instruction that modified the current cell. If we're
/// unsure, or there isn't one, return None.
///
/// Note this totally ignores the instruction at the index given, even
/// if it has an offset. E.g. if the instruction is
/// Set {amount:100, offset: 1}, we're still considering previous instructions that
/// modify the current cell, not the (cell_index + 1)th cell.
fn previous_cell_change(instrs: &[AstNode], index: usize) -> Option<usize> {
    assert!(index < instrs.len());

    let mut needed_offset = 0;
    for i in (0..index).rev() {
        match instrs[i] {
            Increment { offset, .. } | Set { offset, .. } => {
                if offset == needed_offset {
                    return Some(i);
                }
            }
            PointerIncrement { amount, .. } => {
                needed_offset += amount;
            }
            MultiplyMove { ref changes, .. } => {
                // These cells are written to.
                let mut offsets: Vec<isize> = changes.keys().cloned().collect();
                // This cell is zeroed.
                offsets.push(0);

                if offsets.contains(&needed_offset) {
                    return Some(i);
                }
            }
            // No cells changed, so just keep working backwards.
            Write { .. } => {}
            // These instructions may have modified the cell, so
            // we return None for "I don't know".
            Read { .. } | Loop { .. } => return None,
        }
    }
    None
}

/// Inverse of `previous_cell_change`.
///
/// This is very similar to `previous_cell_change` and previous
/// implementations called `previous_cell_change` on the reversed
/// vector. This proved extremely hard to reason about. Instead, we
/// have copied the body of `previous_cell_change` and highlighted the
/// differences.
fn next_cell_change(instrs: &[AstNode], index: usize) -> Option<usize> {
    assert!(index < instrs.len());

    let mut needed_offset = 0;
    // Unlike previous_cell_change, we iterate forward.
    for (i, instr) in instrs.iter().enumerate().skip(index + 1) {
        match *instr {
            Increment { offset, .. } | Set { offset, .. } => {
                if offset == needed_offset {
                    return Some(i);
                }
            }
            PointerIncrement { amount, .. } => {
                // Unlike previous_cell_change we must subtract the desired amount.
                needed_offset -= amount;
            }
            MultiplyMove { ref changes, .. } => {
                // These cells are written to.
                let mut offsets: Vec<isize> = changes.keys().cloned().collect();
                // This cell is zeroed.
                offsets.push(0);

                if offsets.contains(&needed_offset) {
                    return Some(i);
                }
            }
            // No cells changed, so just keep working backwards.
            Write { .. } => {}
            // These instructions may have modified the cell, so
            // we return None for "I don't know".
            Read { .. } | Loop { .. } => return None,
        }
    }
    None
}

/// Combine consecutive increments into a single increment
/// instruction.
fn combine_increments(instrs: Vec<AstNode>) -> Vec<AstNode> {
    instrs
        .into_iter()
        .coalesce(|prev_instr, instr| {
            // Collapse consecutive increments.
            if let Increment {
                amount: prev_amount,
                offset: prev_offset,
                position: prev_pos,
            } = prev_instr
            {
                if let Increment {
                    amount,
                    offset,
                    position,
                } = instr
                {
                    if prev_offset == offset {
                        return Ok(Increment {
                            amount: amount + prev_amount,
                            offset,
                            position: prev_pos.combine(position),
                        });
                    }
                }
            }
            Err((prev_instr, instr))
        })
        .filter(|instr| {
            // Remove any increments of 0.
            if let Increment {
                amount: Wrapping(0),
                ..
            } = *instr
            {
                return false;
            }
            true
        })
        .map_loops(combine_increments)
}

fn combine_ptr_increments(instrs: Vec<AstNode>) -> Vec<AstNode> {
    instrs
        .into_iter()
        .coalesce(|prev_instr, instr| {
            // Collapse consecutive increments.
            if let PointerIncrement {
                amount: prev_amount,
                position: prev_pos,
            } = prev_instr
            {
                if let PointerIncrement { amount, position } = instr {
                    return Ok(PointerIncrement {
                        amount: amount + prev_amount,
                        position: prev_pos.combine(position),
                    });
                }
            }
            Err((prev_instr, instr))
        })
        .filter(|instr| {
            // Remove any pointer increments of 0.
            if let PointerIncrement { amount: 0, .. } = *instr {
                return false;
            }
            true
        })
        .map_loops(combine_ptr_increments)
}

/// Don't bother updating cells if they're immediately overwritten
/// by a value from stdin.
// TODO: this should generate a warning too.
fn remove_read_clobber(instrs: Vec<AstNode>) -> Vec<AstNode> {
    let mut redundant_instr_positions = HashSet::new();
    let mut last_write_index = None;

    for (index, instr) in instrs.iter().enumerate() {
        match *instr {
            Read { .. } => {
                // If we can find the time this cell was modified:
                if let Some(prev_modify_index) = previous_cell_change(&instrs, index) {
                    // This modify instruction is not redundant if we
                    // wrote anything afterwards.
                    if let Some(write_index) = last_write_index {
                        if write_index > prev_modify_index {
                            continue;
                        }
                    }

                    // MultiplyMove instructions are not redundant,
                    // because they affect other cells too.
                    if matches!(instrs[prev_modify_index], MultiplyMove { .. }) {
                        continue;
                    }

                    redundant_instr_positions.insert(prev_modify_index);
                }
            }
            Write { .. } => {
                last_write_index = Some(index);
            }
            _ => {}
        }
    }

    instrs
        .into_iter()
        .enumerate()
        .filter(|&(index, _)| !redundant_instr_positions.contains(&index))
        .map(|(_, instr)| instr)
        .map_loops(remove_read_clobber)
}

/// Convert [-] to Set 0.
fn zeroing_loops(instrs: Vec<AstNode>) -> Vec<AstNode> {
    instrs
        .into_iter()
        .map(|instr| {
            if let Loop { ref body, position } = instr {
                // If the loop is [-]
                if body.len() == 1 {
                    if let Increment {
                        amount: Wrapping(-1),
                        offset: 0,
                        ..
                    } = body[0]
                    {
                        return Set {
                            amount: Wrapping(0),
                            offset: 0,
                            position,
                        };
                    }
                }
            }
            instr
        })
        .map_loops(zeroing_loops)
}

/// Remove any loops where we know the current cell is zero.
fn remove_dead_loops(instrs: Vec<AstNode>) -> Vec<AstNode> {
    instrs
        .clone()
        .into_iter()
        .enumerate()
        .filter(|&(index, ref instr)| {
            if !matches!(instr, Loop { .. }) {
                // Keep all instructions that aren't loops.
                return true;
            }

            // Find the previous change instruction:
            if let Some(prev_change_index) = previous_cell_change(&instrs, index) {
                let prev_instr = &instrs[prev_change_index];
                // If the previous instruction set to zero, our loop is dead.
                // TODO: MultiplyMove also zeroes the current cell.
                // TODO: define an is_set_zero() helper.
                if matches!(
                    prev_instr,
                    Set {
                        amount: Wrapping(0),
                        offset: 0,
                        ..
                    }
                ) {
                    return false;
                }
            }
            true
        })
        .map(|(_, instr)| instr)
        .map_loops(remove_dead_loops)
}

/// Reorder flat sequences of instructions so we use offsets and only
/// have one pointer increment at the end. For example, given "+>+>+<"
/// we return:
/// Increment { amount: 1, offset: 0 }
/// Increment { amount: 1, offset: 1 }
/// Increment { amount: 2, offset: 2 }
/// PointerIncrement(1)
fn sort_by_offset(instrs: Vec<AstNode>) -> Vec<AstNode> {
    let mut sequence = vec![];
    let mut result = vec![];

    for instr in instrs {
        if matches!(
            instr,
            Increment { .. } | Set { .. } | PointerIncrement { .. }
        ) {
            sequence.push(instr);
        } else {
            if !sequence.is_empty() {
                result.extend(sort_sequence_by_offset(sequence));
                sequence = vec![];
            }
            if let Loop { body, position } = instr {
                result.push(Loop {
                    body: sort_by_offset(body),
                    position,
                });
            } else {
                result.push(instr);
            }
        }
    }

    if !sequence.is_empty() {
        result.extend(sort_sequence_by_offset(sequence));
    }

    result
}

/// Given a `HashMap` with orderable keys, return the values according to
/// the key order.
/// {2: 'foo': 1: 'bar'} => vec!['bar', 'foo']
fn ordered_values<K: Ord + Hash + Eq, V>(map: HashMap<K, V>) -> Vec<V> {
    let mut items: Vec<_> = map.into_iter().collect();
    items.sort_by(|a, b| a.0.cmp(&b.0));
    items.into_iter().map(|(_, v)| v).collect()
}

/// Given a BF program, combine sets/increments using offsets so we
/// have single `PointerIncrement` at the end.
fn sort_sequence_by_offset(instrs: Vec<AstNode>) -> Vec<AstNode> {
    let mut instrs_by_offset: HashMap<isize, Vec<AstNode>> = HashMap::new();
    let mut current_offset = 0;
    let mut last_ptr_inc_pos = None;

    for instr in instrs {
        match instr {
            Increment {
                amount,
                offset,
                position,
            } => {
                let new_offset = offset + current_offset;
                let same_offset_instrs = instrs_by_offset.entry(new_offset).or_default();
                same_offset_instrs.push(Increment {
                    amount,
                    offset: new_offset,
                    position,
                });
            }
            Set {
                amount,
                offset,
                position,
            } => {
                let new_offset = offset + current_offset;
                let same_offset_instrs = instrs_by_offset.entry(new_offset).or_default();
                same_offset_instrs.push(Set {
                    amount,
                    offset: new_offset,
                    position,
                });
            }
            PointerIncrement { amount, position } => {
                current_offset += amount;
                last_ptr_inc_pos = Some(position);
            }
            // We assume that we were only given a Vec of
            // Increment/Set/PointerIncrement instructions. It's
            // the job of this function to create instructions with
            // offset.
            _ => unreachable!(),
        }
    }

    // Append the increment/set instructions, in offset order.
    let mut results: Vec<AstNode> = vec![];
    for same_offset_instrs in ordered_values(instrs_by_offset) {
        results.extend(same_offset_instrs.into_iter());
    }

    // Add a single PointerIncrement at the end, reflecting the net
    // pointer movement in this instruction sequence.
    if current_offset != 0 {
        results.push(PointerIncrement {
            amount: current_offset,
            position: last_ptr_inc_pos.unwrap(),
        });
    }
    results
}

/// Combine set instructions with other set instructions or
/// increments.
fn combine_set_and_increments(instrs: Vec<AstNode>) -> Vec<AstNode> {
    // It's sufficient to consider immediately adjacent instructions
    // as sort_sequence_by_offset ensures that if the offset is the
    // same, the instruction is adjacent.
    instrs
        .into_iter()
        .coalesce(|prev_instr, instr| {
            // TODO: Set, Write, Increment -> Set, Write, Set
            // Inc x, Set y -> Set y
            if let (
                &Increment {
                    offset: inc_offset,
                    position: inc_pos,
                    ..
                },
                &Set {
                    amount: set_amount,
                    offset: set_offset,
                    position: set_pos,
                },
            ) = (&prev_instr, &instr)
            {
                if inc_offset == set_offset {
                    return Ok(Set {
                        amount: set_amount,
                        offset: set_offset,
                        // Whilst the Inc is dead here, by including
                        // it in the position tracking we can show better warnings.
                        position: set_pos.combine(inc_pos),
                    });
                }
            }
            Err((prev_instr, instr))
        })
        .coalesce(|prev_instr, instr| {
            // Set x, Inc y -> Set x+y
            if let Set {
                amount: set_amount,
                offset: set_offset,
                position: set_pos,
            } = prev_instr
            {
                if let Increment {
                    amount: inc_amount,
                    offset: inc_offset,
                    position: inc_pos,
                } = instr
                {
                    if inc_offset == set_offset {
                        return Ok(Set {
                            amount: set_amount + inc_amount,
                            offset: set_offset,
                            position: set_pos.combine(inc_pos),
                        });
                    }
                }
            }
            Err((prev_instr, instr))
        })
        .coalesce(|prev_instr, instr| {
            // Set x, Set y -> Set y
            if let (
                &Set {
                    offset: offset1,
                    position: position1,
                    ..
                },
                &Set {
                    amount,
                    offset: offset2,
                    position: position2,
                },
            ) = (&prev_instr, &instr)
            {
                if offset1 == offset2 {
                    return Ok(Set {
                        amount,
                        offset: offset1,
                        // Whilst the first Set is dead here, by including
                        // it in the position tracking we can show better warnings.
                        position: position1.combine(position2),
                    });
                }
            }
            Err((prev_instr, instr))
        })
        .map_loops(combine_set_and_increments)
}

fn remove_redundant_sets(instrs: Vec<AstNode>) -> Vec<AstNode> {
    let mut reduced = remove_redundant_sets_inner(instrs);

    // Remove a set zero at the beginning of the program, since cells
    // are initialised to zero anyway.
    if matches!(
        reduced.first(),
        Some(Set {
            amount: Wrapping(0),
            offset: 0,
            ..
        })
    ) {
        reduced.remove(0);
    }

    reduced
}

fn remove_redundant_sets_inner(instrs: Vec<AstNode>) -> Vec<AstNode> {
    let mut redundant_instr_positions = HashSet::new();

    for (index, instr) in instrs.iter().enumerate() {
        if matches!(instr, Loop { .. } | MultiplyMove { .. }) {
            // There's no point setting to zero after a loop, as
            // the cell is already zero.
            if let Some(next_index) = next_cell_change(&instrs, index) {
                if let Set {
                    amount: Wrapping(0),
                    offset: 0,
                    ..
                } = instrs[next_index]
                {
                    redundant_instr_positions.insert(next_index);
                }
            }
        }
    }

    instrs
        .into_iter()
        .enumerate()
        .filter(|&(index, _)| !redundant_instr_positions.contains(&index))
        .map(|(_, instr)| instr)
        .map_loops(remove_redundant_sets_inner)
}

fn annotate_known_zero(instrs: Vec<AstNode>) -> Vec<AstNode> {
    let mut result = vec![];

    let position = if instrs.is_empty() {
        None
    } else {
        get_position(&instrs[0]).map(|first_instr_pos| Position {
            start: first_instr_pos.start,
            end: first_instr_pos.start,
        })
    };

    // Cells in BF are initialised to zero, so we know the current
    // cell is zero at the start of execution.
    let set_instr = Set {
        amount: Wrapping(0),
        offset: 0,
        position,
    };
    // Insert the set instruction unless there is one already present.
    if instrs.first() != Some(&set_instr) {
        result.push(set_instr);
    }

    result.extend(annotate_known_zero_inner(&instrs));
    result
}

fn annotate_known_zero_inner(instrs: &[AstNode]) -> Vec<AstNode> {
    let mut result = Vec::with_capacity(instrs.len());

    for (i, instr) in instrs.iter().enumerate() {
        let instr = instr.clone();

        match instr {
            // After a loop, we know the cell is currently zero.
            Loop { body, position } => {
                result.push(Loop {
                    body: annotate_known_zero_inner(&body),
                    position,
                });
                // Treat this set as positioned at the ].
                let set_pos = position.map(|loop_pos| Position {
                    start: loop_pos.end,
                    end: loop_pos.end,
                });

                let set_instr = Set {
                    amount: Wrapping(0),
                    offset: 0,
                    position: set_pos,
                };
                if instrs.get(i + 1) != Some(&set_instr) {
                    result.push(set_instr.clone());
                }
            }
            _ => {
                result.push(instr);
            }
        }
    }

    result
}

/// Remove code at the end of the program that has no side
/// effects. This means we have no write commands afterwards, nor
/// loops (which may not terminate so we should not remove).
fn remove_pure_code(mut instrs: Vec<AstNode>) -> (Vec<AstNode>, Option<Warning>) {
    let mut pure_instrs = vec![];

    while let Some(last_instr) = instrs.pop() {
        match last_instr {
            Read { .. } | Write { .. } | Loop { .. } => {
                instrs.push(last_instr);
                break;
            }
            _ => {
                pure_instrs.push(last_instr);
            }
        }
    }

    let warning = if pure_instrs.is_empty() {
        None
    } else {
        let position = pure_instrs
            .into_iter()
            .map(|instr| get_position(&instr))
            .filter(|pos| pos.is_some())
            .reduce(|pos1, pos2| pos1.combine(pos2))
            .map(|pos| pos.unwrap());
        Some(Warning {
            message: "These instructions have no effect.".to_owned(),
            position,
        })
    };

    (instrs, warning)
}

/// Does this loop body represent a multiplication operation?
/// E.g. "[->>>++<<<]" sets cell #3 to 2*cell #0.
fn is_multiply_loop_body(body: &[AstNode]) -> bool {
    // A multiply loop may only contain increments and pointer increments.
    for body_instr in body {
        match *body_instr {
            Increment { .. } | PointerIncrement { .. } => {}
            _ => return false,
        }
    }

    // A multiply loop must have a net pointer movement of
    // zero.
    let mut net_movement = 0;
    for body_instr in body {
        if let PointerIncrement { amount, .. } = *body_instr {
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
fn cell_changes(instrs: &[AstNode]) -> HashMap<isize, BfValue> {
    let mut changes = HashMap::new();
    let mut cell_index: isize = 0;

    for instr in instrs {
        match *instr {
            Increment { amount, offset, .. } => {
                let current_amount = *changes.get(&(cell_index + offset)).unwrap_or(&Wrapping(0));
                changes.insert(cell_index, current_amount + amount);
            }
            PointerIncrement { amount, .. } => {
                cell_index += amount;
            }
            // We assume this is only called from is_multiply_loop.
            _ => unreachable!(),
        }
    }

    changes
}

fn extract_multiply(instrs: Vec<AstNode>) -> Vec<AstNode> {
    instrs
        .into_iter()
        .map(|instr| {
            match instr {
                Loop { body, position } => {
                    if is_multiply_loop_body(&body) {
                        let mut changes = cell_changes(&body);
                        // MultiplyMove is for where we move to, so ignore
                        // the cell we're moving from.
                        changes.remove(&0);

                        MultiplyMove { changes, position }
                    } else {
                        Loop {
                            body: extract_multiply(body),
                            position,
                        }
                    }
                }
                i => i,
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::collections::HashMap;
    use std::num::Wrapping;

    use pretty_assertions::assert_eq;
    use quickcheck::quickcheck;
    use quickcheck::{Arbitrary, Gen, TestResult};

    use crate::bfir::parse;
    use crate::bfir::{AstNode, Position};
    use crate::diagnostics::Warning;

    impl Arbitrary for AstNode {
        fn arbitrary<G: Gen>(g: &mut G) -> AstNode {
            arbitrary_instr(g, 5)
        }
    }

    // We define a separate function so we can recurse on max_depth.
    // See https://github.com/BurntSushi/quickcheck/issues/23
    fn arbitrary_instr<G: Gen>(g: &mut G, max_depth: usize) -> AstNode {
        let modulus = if max_depth == 0 { 8 } else { 9 };

        // If max_depth is zero, don't create loops.
        match g.next_u32() % modulus {
            // TODO: use arbitrary offsets.
            0 => Increment {
                amount: Wrapping(Arbitrary::arbitrary(g)),
                offset: 0,
                position: Some(Position { start: 0, end: 0 }),
            },
            1 => PointerIncrement {
                amount: Arbitrary::arbitrary(g),
                position: Some(Position { start: 0, end: 0 }),
            },
            // TODO: use arbitrary offsets.
            2 => Set {
                amount: Wrapping(Arbitrary::arbitrary(g)),
                offset: 0,
                position: Some(Position { start: 0, end: 0 }),
            },
            3 => Read {
                position: Some(Position { start: 0, end: 0 }),
            },
            4 => Write {
                position: Some(Position { start: 0, end: 0 }),
            },
            5 => {
                let mut changes = HashMap::new();
                changes.insert(1, Wrapping(-1));
                MultiplyMove {
                    changes,
                    position: Some(Position { start: 0, end: 0 }),
                }
            }
            6 => {
                let mut changes = HashMap::new();
                changes.insert(1, Wrapping(2));
                changes.insert(4, Wrapping(10));
                MultiplyMove {
                    changes,
                    position: Some(Position { start: 0, end: 0 }),
                }
            }
            7 => {
                // A multiply by 2 loop that accesses a previous
                // cell. Quickcheck doesn't seem to generate these by
                // chance, but they often expose interesting bugs.
                let body = vec![
                    Increment {
                        amount: Wrapping(-1),
                        offset: 0,
                        position: None,
                    },
                    PointerIncrement {
                        amount: -1,
                        position: None,
                    },
                    Increment {
                        amount: Wrapping(2),
                        offset: 0,
                        position: None,
                    },
                    PointerIncrement {
                        amount: 1,
                        position: None,
                    },
                ];
                Loop {
                    body,
                    position: None,
                }
            }
            8 => {
                assert!(max_depth > 0);
                let loop_length = g.next_u32() % 10;
                let mut body: Vec<_> = vec![];
                for _ in 0..loop_length {
                    body.push(arbitrary_instr(g, max_depth - 1));
                }
                Loop {
                    body,
                    position: Some(Position { start: 0, end: 0 }),
                }
            }
            _ => unreachable!(),
        }
    }

    #[test]
    fn combine_increments_flat() {
        let initial = parse("++").unwrap();
        let expected = vec![Increment {
            amount: Wrapping(2),
            offset: 0,
            position: Some(Position { start: 0, end: 1 }),
        }];
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
        let expected = vec![Loop {
            body: vec![Increment {
                amount: Wrapping(2),
                offset: 0,
                position: Some(Position { start: 1, end: 2 }),
            }],
            position: Some(Position { start: 0, end: 3 }),
        }];
        assert_eq!(combine_increments(initial), expected);
    }

    #[test]
    fn combine_increments_remove_redundant() {
        let initial = parse("+-").unwrap();
        assert_eq!(combine_increments(initial), vec![]);
    }

    #[test]
    fn quickcheck_combine_increments_remove_zero_any_offset() {
        fn combine_increments_remove_zero_any_offset(offset: isize) -> bool {
            let initial = vec![Increment {
                amount: Wrapping(0),
                offset,
                position: Some(Position { start: 0, end: 0 }),
            }];
            combine_increments(initial) == vec![]
        }
        quickcheck(combine_increments_remove_zero_any_offset as fn(isize) -> bool);
    }

    #[test]
    fn combine_increment_sum_to_zero() {
        let initial = vec![
            Increment {
                amount: Wrapping(-1),
                offset: 0,
                position: Some(Position { start: 0, end: 0 }),
            },
            Increment {
                amount: Wrapping(1),
                offset: 0,
                position: Some(Position { start: 0, end: 0 }),
            },
        ];
        assert_eq!(combine_increments(initial), vec![]);
    }

    #[test]
    fn should_combine_ptr_increments() {
        let initial = parse(">>").unwrap();
        let expected = vec![PointerIncrement {
            amount: 2,
            position: Some(Position { start: 0, end: 1 }),
        }];
        assert_eq!(combine_ptr_increments(initial), expected);
    }

    #[test]
    fn combine_set_sum_to_zero() {
        let initial = vec![
            Set {
                amount: Wrapping(-1),
                offset: 0,
                position: Some(Position { start: 0, end: 0 }),
            },
            Increment {
                amount: Wrapping(1),
                offset: 0,
                position: Some(Position { start: 0, end: 0 }),
            },
        ];
        assert_eq!(
            combine_set_and_increments(initial),
            vec![Set {
                amount: Wrapping(0),
                offset: 0,
                position: Some(Position { start: 0, end: 0 }),
            }]
        );
    }

    #[test]
    fn should_combine_before_read() {
        // The increment before the read is dead and can be removed.
        let initial = parse("+,.").unwrap();
        let expected = vec![
            Read {
                position: Some(Position { start: 1, end: 1 }),
            },
            Write {
                position: Some(Position { start: 2, end: 2 }),
            },
        ];
        assert_eq!(optimize(initial, &None).0, expected);
    }

    #[test]
    fn dont_combine_before_read_different_offset() {
        // The read does not affect the increment here.
        let initial = vec![
            Increment {
                amount: Wrapping(1),
                offset: 2,
                position: Some(Position { start: 0, end: 0 }),
            },
            Read {
                position: Some(Position { start: 0, end: 0 }),
            },
        ];
        assert_eq!(remove_read_clobber(initial.clone()), initial);
    }

    #[test]
    fn should_combine_before_read_nested() {
        // The read clobbers the increment here.
        let initial = parse("+[+,]").unwrap();
        let expected = vec![
            Set {
                amount: Wrapping(1),
                offset: 0,
                position: Some(Position { start: 0, end: 0 }),
            },
            Loop {
                body: vec![Read {
                    position: Some(Position { start: 3, end: 3 }),
                }],
                position: Some(Position { start: 1, end: 4 }),
            },
        ];
        assert_eq!(optimize(initial, &None).0, expected);
    }

    #[test]
    fn combine_before_read_not_consecutive() {
        // The increment before the read is dead and can be removed.
        let initial = parse("+>-<,").unwrap();
        let expected = vec![
            PointerIncrement {
                amount: 1,
                position: Some(Position { start: 1, end: 1 }),
            },
            Increment {
                amount: Wrapping(-1),
                offset: 0,
                position: Some(Position { start: 2, end: 2 }),
            },
            PointerIncrement {
                amount: -1,
                position: Some(Position { start: 3, end: 3 }),
            },
            Read {
                position: Some(Position { start: 4, end: 4 }),
            },
        ];
        assert_eq!(remove_read_clobber(initial), expected);
    }

    #[test]
    fn no_combine_before_read_after_write() {
        let initial = vec![
            Set {
                amount: Wrapping(1),
                offset: 0,
                position: None,
            },
            Write { position: None },
            Read { position: None },
        ];
        // TODO: write an assert_unchanged! macro.
        let expected = initial.clone();
        assert_eq!(remove_read_clobber(initial), expected);
    }

    #[test]
    fn no_combine_before_read_after_multiply() {
        let mut changes = HashMap::new();
        changes.insert(1, Wrapping(-1));
        let initial = vec![
            MultiplyMove {
                changes,
                position: None,
            },
            Read { position: None },
        ];
        let expected = initial.clone();
        assert_eq!(remove_read_clobber(initial), expected);
    }

    #[test]
    fn simplify_zeroing_loop() {
        let initial = parse("[-]").unwrap();
        let expected = vec![Set {
            amount: Wrapping(0),
            offset: 0,
            position: Some(Position { start: 0, end: 2 }),
        }];
        assert_eq!(zeroing_loops(initial), expected);
    }

    #[test]
    fn simplify_nested_zeroing_loop() {
        let initial = parse("[[-]]").unwrap();
        let expected = vec![Loop {
            body: vec![Set {
                amount: Wrapping(0),
                offset: 0,
                position: Some(Position { start: 1, end: 3 }),
            }],
            position: Some(Position { start: 0, end: 4 }),
        }];
        assert_eq!(zeroing_loops(initial), expected);
    }

    #[test]
    fn dont_simplify_multiple_decrement_loop() {
        // A user who wrote this probably meant '[-]'. However, if the
        // current cell has the value 3, we would actually wrap around
        // (although BF does not specify this).
        let initial = parse("[--]").unwrap();
        assert_eq!(zeroing_loops(initial.clone()), initial);
    }

    #[test]
    fn remove_repeated_loops() {
        let initial = vec![
            Set {
                amount: Wrapping(1),
                offset: 0,
                position: Some(Position { start: 0, end: 0 }),
            },
            Loop {
                body: vec![],
                position: Some(Position { start: 0, end: 0 }),
            },
            Loop {
                body: vec![],
                position: Some(Position { start: 0, end: 0 }),
            },
        ];
        let expected = vec![
            Set {
                amount: Wrapping(1),
                offset: 0,
                position: Some(Position { start: 0, end: 0 }),
            },
            Loop {
                body: vec![],
                position: Some(Position { start: 0, end: 0 }),
            },
        ];
        assert_eq!(optimize(initial, &None).0, expected);
    }

    #[test]
    fn remove_dead_loops_after_set() {
        let initial = vec![
            Set {
                amount: Wrapping(0),
                offset: 0,
                position: Some(Position { start: 0, end: 0 }),
            },
            Loop {
                body: vec![],
                position: Some(Position { start: 0, end: 0 }),
            },
        ];
        let expected = vec![Set {
            amount: Wrapping(0),
            offset: 0,
            position: Some(Position { start: 0, end: 0 }),
        }];
        assert_eq!(remove_dead_loops(initial), expected);
    }

    #[test]
    fn remove_dead_loops_nested() {
        let initial = vec![Loop {
            body: vec![
                Set {
                    amount: Wrapping(0),
                    offset: 0,
                    position: Some(Position { start: 0, end: 0 }),
                },
                Loop {
                    body: vec![],
                    position: Some(Position { start: 0, end: 0 }),
                },
            ],
            position: Some(Position { start: 0, end: 0 }),
        }];
        let expected = vec![Loop {
            body: vec![Set {
                amount: Wrapping(0),
                offset: 0,
                position: Some(Position { start: 0, end: 0 }),
            }],
            position: Some(Position { start: 0, end: 0 }),
        }];
        assert_eq!(remove_dead_loops(initial), expected);
    }

    #[test]
    fn remove_dead_loops_not_adjacent() {
        let initial = vec![
            Set {
                amount: Wrapping(0),
                offset: 0,
                position: Some(Position { start: 0, end: 0 }),
            },
            Set {
                amount: Wrapping(1),
                offset: 1,
                position: Some(Position { start: 0, end: 0 }),
            },
            Loop {
                body: vec![],
                position: Some(Position { start: 0, end: 0 }),
            },
        ];
        let expected = vec![
            Set {
                amount: Wrapping(0),
                offset: 0,
                position: Some(Position { start: 0, end: 0 }),
            },
            Set {
                amount: Wrapping(1),
                offset: 1,
                position: Some(Position { start: 0, end: 0 }),
            },
        ];
        assert_eq!(remove_dead_loops(initial), expected);
    }

    #[test]
    fn quickcheck_should_combine_set_and_increment() {
        fn should_combine_set_and_increment(
            offset: isize,
            set_amount: i8,
            increment_amount: i8,
        ) -> bool {
            let set_amount = Wrapping(set_amount);
            let increment_amount = Wrapping(increment_amount);

            let initial = vec![
                Set {
                    amount: set_amount,
                    offset,
                    position: Some(Position { start: 0, end: 0 }),
                },
                Increment {
                    amount: increment_amount,
                    offset,
                    position: Some(Position { start: 0, end: 0 }),
                },
            ];
            let expected = vec![Set {
                amount: set_amount + increment_amount,
                offset,
                position: Some(Position { start: 0, end: 0 }),
            }];
            combine_set_and_increments(initial) == expected
        }
        quickcheck(should_combine_set_and_increment as fn(isize, i8, i8) -> bool);
    }

    // TODO: rename our quickcheck property functions to something shorter.
    #[test]
    fn quickcheck_combine_set_and_increment_different_offsets() {
        fn combine_set_and_increment_different_offsets(
            set_offset: isize,
            set_amount: i8,
            inc_offset: isize,
            inc_amount: i8,
        ) -> TestResult {
            if set_offset == inc_offset {
                return TestResult::discard();
            }

            let initial = vec![
                Set {
                    amount: Wrapping(set_amount),
                    offset: set_offset,
                    position: Some(Position { start: 0, end: 0 }),
                },
                Increment {
                    amount: Wrapping(inc_amount),
                    offset: inc_offset,
                    position: Some(Position { start: 0, end: 0 }),
                },
            ];
            let expected = initial.clone();

            TestResult::from_bool(combine_set_and_increments(initial) == expected)
        }
        quickcheck(
            combine_set_and_increment_different_offsets as fn(isize, i8, isize, i8) -> TestResult,
        );
    }

    #[test]
    fn quickcheck_combine_increment_and_set_different_offsets() {
        fn combine_increment_and_set_different_offsets(
            set_offset: isize,
            set_amount: i8,
            inc_offset: isize,
            inc_amount: i8,
        ) -> TestResult {
            if set_offset == inc_offset {
                return TestResult::discard();
            }

            let initial = vec![
                Increment {
                    amount: Wrapping(inc_amount),
                    offset: inc_offset,
                    position: Some(Position { start: 0, end: 0 }),
                },
                Set {
                    amount: Wrapping(set_amount),
                    offset: set_offset,
                    position: Some(Position { start: 0, end: 0 }),
                },
            ];
            let expected = initial.clone();

            TestResult::from_bool(combine_set_and_increments(initial) == expected)
        }
        quickcheck(
            combine_increment_and_set_different_offsets as fn(isize, i8, isize, i8) -> TestResult,
        );
    }

    #[test]
    fn quickcheck_combine_set_and_set() {
        fn combine_set_and_set(offset: isize, set_amount_before: i8, set_amount_after: i8) -> bool {
            let initial = vec![
                Set {
                    amount: Wrapping(set_amount_before),
                    offset,
                    position: Some(Position { start: 0, end: 0 }),
                },
                Set {
                    amount: Wrapping(set_amount_after),
                    offset,
                    position: Some(Position { start: 0, end: 0 }),
                },
            ];
            let expected = vec![Set {
                amount: Wrapping(set_amount_after),
                offset,
                position: Some(Position { start: 0, end: 0 }),
            }];
            combine_set_and_increments(initial) == expected
        }
        quickcheck(combine_set_and_set as fn(isize, i8, i8) -> bool);
    }

    #[test]
    fn quickcheck_combine_set_and_set_different_offsets() {
        fn combine_set_and_set_different_offsets(
            offset1: isize,
            amount1: i8,
            offset2: isize,
            amount2: i8,
        ) -> TestResult {
            if offset1 == offset2 {
                return TestResult::discard();
            }

            let initial = vec![
                Set {
                    amount: Wrapping(amount1),
                    offset: offset1,
                    position: Some(Position { start: 0, end: 0 }),
                },
                Set {
                    amount: Wrapping(amount2),
                    offset: offset2,
                    position: Some(Position { start: 0, end: 0 }),
                },
            ];
            let expected = initial.clone();

            TestResult::from_bool(combine_set_and_increments(initial) == expected)
        }
        quickcheck(combine_set_and_set_different_offsets as fn(isize, i8, isize, i8) -> TestResult);
    }

    #[test]
    fn should_combine_set_and_set_nested() {
        let initial = vec![Loop {
            body: vec![
                Set {
                    amount: Wrapping(0),
                    offset: 0,
                    position: Some(Position { start: 0, end: 0 }),
                },
                Set {
                    amount: Wrapping(1),
                    offset: 0,
                    position: Some(Position { start: 0, end: 0 }),
                },
            ],
            position: Some(Position { start: 0, end: 0 }),
        }];
        let expected = vec![Loop {
            body: vec![Set {
                amount: Wrapping(1),
                offset: 0,
                position: Some(Position { start: 0, end: 0 }),
            }],
            position: Some(Position { start: 0, end: 0 }),
        }];
        assert_eq!(combine_set_and_increments(initial), expected);
    }

    #[test]
    fn quickcheck_should_combine_increment_and_set() {
        fn should_combine_increment_and_set(offset: isize) -> bool {
            let initial = vec![
                Increment {
                    amount: Wrapping(2),
                    offset,
                    position: Some(Position { start: 0, end: 0 }),
                },
                Set {
                    amount: Wrapping(3),
                    offset,
                    position: Some(Position { start: 0, end: 0 }),
                },
            ];
            let expected = vec![Set {
                amount: Wrapping(3),
                offset,
                position: Some(Position { start: 0, end: 0 }),
            }];
            combine_set_and_increments(initial) == expected
        }
        quickcheck(should_combine_increment_and_set as fn(isize) -> bool);
    }

    #[test]
    fn should_remove_redundant_set() {
        let initial = vec![
            Loop {
                body: vec![],
                position: Some(Position { start: 0, end: 0 }),
            },
            Set {
                amount: Wrapping(0),
                offset: -1,
                position: Some(Position { start: 0, end: 0 }),
            },
            Set {
                amount: Wrapping(0),
                offset: 0,
                position: Some(Position { start: 0, end: 0 }),
            },
        ];
        let expected = vec![
            Loop {
                body: vec![],
                position: Some(Position { start: 0, end: 0 }),
            },
            Set {
                amount: Wrapping(0),
                offset: -1,
                position: Some(Position { start: 0, end: 0 }),
            },
        ];
        assert_eq!(remove_redundant_sets(initial), expected);
    }

    #[test]
    fn should_remove_redundant_set_multiply() {
        let mut changes = HashMap::new();
        changes.insert(1, Wrapping(1));

        let initial = vec![
            MultiplyMove {
                changes: changes.clone(),
                position: Some(Position { start: 0, end: 0 }),
            },
            Set {
                amount: Wrapping(0),
                offset: 0,
                position: Some(Position { start: 0, end: 0 }),
            },
        ];
        let expected = vec![MultiplyMove {
            changes,
            position: Some(Position { start: 0, end: 0 }),
        }];
        assert_eq!(remove_redundant_sets(initial), expected);
    }

    /// After a loop, if we set to a value other than zero, we shouldn't
    /// remove it.
    #[test]
    fn not_redundant_set_when_nonzero() {
        let instrs = vec![
            Loop {
                body: vec![],
                position: Some(Position { start: 0, end: 0 }),
            },
            Set {
                amount: Wrapping(1),
                offset: 0,
                position: Some(Position { start: 0, end: 0 }),
            },
        ];
        assert_eq!(remove_redundant_sets(instrs.clone()), instrs);
    }

    fn is_pure(instrs: &[AstNode]) -> bool {
        for instr in instrs {
            match *instr {
                Loop { .. } => {
                    return false;
                }
                Read { .. } => {
                    return false;
                }
                Write { .. } => {
                    return false;
                }
                _ => (),
            }
        }
        true
    }

    #[test]
    fn quickcheck_should_annotate_known_zero_at_start() {
        fn should_annotate_known_zero_at_start(instrs: Vec<AstNode>) -> bool {
            let annotated = annotate_known_zero(instrs);
            matches!(
                annotated[0],
                Set {
                    amount: Wrapping(0),
                    offset: 0,
                    ..
                }
            )
        }
        quickcheck(should_annotate_known_zero_at_start as fn(Vec<AstNode>) -> bool);
    }

    #[test]
    fn annotate_known_zero_idempotent() {
        fn is_idempotent(instrs: Vec<AstNode>) -> bool {
            let annotated = annotate_known_zero(instrs);
            let annotated_again = annotate_known_zero(annotated.clone());
            if annotated == annotated_again {
                true
            } else {
                println!("intermediate: {:?}", annotated);
                println!("final: {:?}", annotated_again);
                false
            }
        }
        quickcheck(is_idempotent as fn(Vec<AstNode>) -> bool);
    }

    #[test]
    fn should_annotate_known_zero() {
        let initial = parse("+[]").unwrap();
        let expected = vec![
            Set {
                amount: Wrapping(0),
                offset: 0,
                position: Some(Position { start: 0, end: 0 }),
            },
            Increment {
                amount: Wrapping(1),
                offset: 0,
                position: Some(Position { start: 0, end: 0 }),
            },
            Loop {
                body: vec![],
                position: Some(Position { start: 1, end: 2 }),
            },
            Set {
                amount: Wrapping(0),
                offset: 0,
                position: Some(Position { start: 2, end: 2 }),
            },
        ];
        assert_eq!(annotate_known_zero(initial), expected);
    }

    #[test]
    fn should_annotate_known_zero_nested() {
        let initial = parse("[[]]").unwrap();
        let expected = vec![
            Set {
                amount: Wrapping(0),
                offset: 0,
                position: Some(Position { start: 0, end: 0 }),
            },
            Loop {
                body: vec![
                    Loop {
                        body: vec![],
                        position: Some(Position { start: 1, end: 2 }),
                    },
                    Set {
                        amount: Wrapping(0),
                        offset: 0,
                        position: Some(Position { start: 2, end: 2 }),
                    },
                ],
                position: Some(Position { start: 0, end: 3 }),
            },
            Set {
                amount: Wrapping(0),
                offset: 0,
                position: Some(Position { start: 3, end: 3 }),
            },
        ];
        assert_eq!(annotate_known_zero(initial), expected);
    }

    /// When we annotate known zeroes, we have new opportunities for
    /// combining instructions and loop removal. However, we should later
    /// remove the Set 0 if we haven't combined it.
    #[test]
    fn should_annotate_known_zero_cleaned_up() {
        let initial = vec![Write {
            position: Some(Position { start: 0, end: 0 }),
        }];
        assert_eq!(optimize(initial.clone(), &None).0, initial);
    }

    #[test]
    fn should_preserve_set_0_in_loop() {
        // Regression test.
        let initial = vec![
            Read {
                position: Some(Position { start: 0, end: 0 }),
            },
            Loop {
                body: vec![Set {
                    amount: Wrapping(0),
                    offset: 0,
                    position: Some(Position { start: 0, end: 0 }),
                }],
                position: Some(Position { start: 0, end: 0 }),
            },
        ];
        assert_eq!(optimize(initial.clone(), &None).0, initial);
    }

    #[test]
    fn should_remove_pure_code() {
        // The final increment here is side-effect free and can be
        // removed.
        let initial = parse("+.+").unwrap();
        let expected = vec![
            Set {
                amount: Wrapping(1),
                offset: 0,
                position: Some(Position { start: 0, end: 0 }),
            },
            Write {
                position: Some(Position { start: 1, end: 1 }),
            },
        ];

        let (result, warnings) = optimize(initial, &None);

        assert_eq!(result, expected);
        assert_eq!(
            warnings,
            vec![Warning {
                message: "These instructions have no effect.".to_owned(),
                position: Some(Position { start: 2, end: 2 }),
            }]
        );
    }

    #[test]
    fn quickcheck_should_remove_dead_pure_code() {
        fn should_remove_dead_pure_code(instrs: Vec<AstNode>) -> TestResult {
            if !is_pure(&instrs) {
                return TestResult::discard();
            }
            TestResult::from_bool(optimize(instrs, &None).0 == vec![])
        }
        quickcheck(should_remove_dead_pure_code as fn(Vec<AstNode>) -> TestResult);
    }

    #[test]
    fn quickcheck_optimize_should_be_idempotent() {
        fn optimize_should_be_idempotent(instrs: Vec<AstNode>) -> bool {
            // Once we've optimized once, running again shouldn't reduce the
            // instructions further. If it does, we're probably running our
            // optimisations in the wrong order.
            let minimal = optimize(instrs, &None).0;
            optimize(minimal.clone(), &None).0 == minimal
        }
        quickcheck(optimize_should_be_idempotent as fn(Vec<AstNode>) -> bool);
    }

    #[test]
    fn pathological_optimisation_opportunity() {
        let instrs = vec![
            Read {
                position: Some(Position { start: 0, end: 0 }),
            },
            Increment {
                amount: Wrapping(1),
                offset: 0,
                position: Some(Position { start: 0, end: 0 }),
            },
            PointerIncrement {
                amount: 1,
                position: Some(Position { start: 0, end: 0 }),
            },
            Increment {
                amount: Wrapping(1),
                offset: 0,
                position: Some(Position { start: 0, end: 0 }),
            },
            PointerIncrement {
                amount: 1,
                position: Some(Position { start: 0, end: 0 }),
            },
            PointerIncrement {
                amount: -1,
                position: Some(Position { start: 0, end: 0 }),
            },
            Increment {
                amount: Wrapping(-1),
                offset: 0,
                position: Some(Position { start: 0, end: 0 }),
            },
            PointerIncrement {
                amount: -1,
                position: Some(Position { start: 0, end: 0 }),
            },
            Increment {
                amount: Wrapping(-1),
                offset: 0,
                position: Some(Position { start: 0, end: 0 }),
            },
            Write {
                position: Some(Position { start: 0, end: 0 }),
            },
        ];

        let expected = vec![
            Read {
                position: Some(Position { start: 0, end: 0 }),
            },
            Write {
                position: Some(Position { start: 0, end: 0 }),
            },
        ];

        assert_eq!(optimize(instrs, &None).0, expected);
    }

    fn count_instrs(instrs: &[AstNode]) -> u64 {
        let mut count = 0;
        for instr in instrs {
            if let Loop { ref body, .. } = *instr {
                count += count_instrs(body);
            }
            count += 1;
        }
        count
    }

    #[test]
    fn quickcheck_optimize_should_decrease_size() {
        fn optimize_should_decrease_size(instrs: Vec<AstNode>) -> bool {
            // The result of optimize() should never increase the number of
            // instructions.
            let result = optimize(instrs.clone(), &None).0;
            count_instrs(&result) <= count_instrs(&instrs)
        }
        quickcheck(optimize_should_decrease_size as fn(Vec<AstNode>) -> bool);
    }

    #[test]
    fn should_extract_multiply_simple() {
        let instrs = parse("[->+++<]").unwrap();

        let mut dest_cells = HashMap::new();
        dest_cells.insert(1, Wrapping(3));
        let expected = vec![MultiplyMove {
            changes: dest_cells,
            position: Some(Position { start: 0, end: 7 }),
        }];

        assert_eq!(extract_multiply(instrs), expected);
    }

    #[test]
    fn should_extract_multiply_nested() {
        let instrs = parse("[[->+<]]").unwrap();

        let mut dest_cells = HashMap::new();
        dest_cells.insert(1, Wrapping(1));
        let expected = vec![Loop {
            body: vec![MultiplyMove {
                changes: dest_cells,
                position: Some(Position { start: 1, end: 6 }),
            }],
            position: Some(Position { start: 0, end: 7 }),
        }];

        assert_eq!(extract_multiply(instrs), expected);
    }

    #[test]
    fn should_extract_multiply_negative_number() {
        let instrs = parse("[->--<]").unwrap();

        let mut dest_cells = HashMap::new();
        dest_cells.insert(1, Wrapping(-2));
        let expected = vec![MultiplyMove {
            changes: dest_cells,
            position: Some(Position { start: 0, end: 6 }),
        }];

        assert_eq!(extract_multiply(instrs), expected);
    }

    #[test]
    fn should_extract_multiply_multiple_cells() {
        let instrs = parse("[->+++>>>+<<<<]").unwrap();

        let mut dest_cells = HashMap::new();
        dest_cells.insert(1, Wrapping(3));
        dest_cells.insert(4, Wrapping(1));
        let expected = vec![MultiplyMove {
            changes: dest_cells,
            position: Some(Position { start: 0, end: 14 }),
        }];

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

    #[test]
    fn should_not_extract_multiply_with_read() {
        let instrs = parse("[+>++<,]").unwrap();
        assert_eq!(extract_multiply(instrs.clone()), instrs);
    }

    #[test]
    fn should_not_extract_multiply_with_write() {
        let instrs = parse("[+>++<.]").unwrap();
        assert_eq!(extract_multiply(instrs.clone()), instrs);
    }

    #[test]
    fn sort_by_offset_increment() {
        let instrs = parse("+>+>").unwrap();
        let expected = vec![
            Increment {
                amount: Wrapping(1),
                offset: 0,
                position: Some(Position { start: 0, end: 0 }),
            },
            Increment {
                amount: Wrapping(1),
                offset: 1,
                position: Some(Position { start: 2, end: 2 }),
            },
            PointerIncrement {
                amount: 2,
                position: Some(Position { start: 3, end: 3 }),
            },
        ];
        assert_eq!(sort_by_offset(instrs), expected);
    }

    #[test]
    fn sort_by_offset_increment_nested() {
        let instrs = parse("[+>+>]").unwrap();
        let expected = vec![Loop {
            body: (vec![
                Increment {
                    amount: Wrapping(1),
                    offset: 0,
                    position: Some(Position { start: 1, end: 1 }),
                },
                Increment {
                    amount: Wrapping(1),
                    offset: 1,
                    position: Some(Position { start: 3, end: 3 }),
                },
                PointerIncrement {
                    amount: 2,
                    position: Some(Position { start: 4, end: 4 }),
                },
            ]),
            position: Some(Position { start: 0, end: 5 }),
        }];
        assert_eq!(sort_by_offset(instrs), expected);
    }

    #[test]
    fn sort_by_offset_remove_redundant() {
        let initial = parse("><").unwrap();
        assert_eq!(sort_by_offset(initial), vec![]);
    }

    // If there's a read instruction, we should only combine before and
    // after.
    #[test]
    fn sort_by_offset_read() {
        let instrs = parse(">>,>>").unwrap();
        let expected = vec![
            PointerIncrement {
                amount: 2,
                position: Some(Position { start: 1, end: 1 }),
            },
            Read {
                position: Some(Position { start: 2, end: 2 }),
            },
            PointerIncrement {
                amount: 2,
                position: Some(Position { start: 4, end: 4 }),
            },
        ];
        assert_eq!(sort_by_offset(instrs), expected);
    }

    #[test]
    fn quickcheck_sort_by_offset_set() {
        fn sort_by_offset_set(amount1: i8, amount2: i8) -> bool {
            let instrs = vec![
                Set {
                    amount: Wrapping(amount1),
                    offset: 0,
                    position: Some(Position { start: 0, end: 0 }),
                },
                PointerIncrement {
                    amount: -1,
                    position: Some(Position { start: 0, end: 0 }),
                },
                Set {
                    amount: Wrapping(amount2),
                    offset: 0,
                    position: Some(Position { start: 0, end: 0 }),
                },
            ];

            let expected = vec![
                Set {
                    amount: Wrapping(amount2),
                    offset: -1,
                    position: Some(Position { start: 0, end: 0 }),
                },
                Set {
                    amount: Wrapping(amount1),
                    offset: 0,
                    position: Some(Position { start: 0, end: 0 }),
                },
                PointerIncrement {
                    amount: -1,
                    position: Some(Position { start: 0, end: 0 }),
                },
            ];
            sort_by_offset(instrs) == expected
        }
        quickcheck(sort_by_offset_set as fn(i8, i8) -> bool);
    }

    #[test]
    fn quickcheck_sort_by_offset_pointer_increments() {
        fn sort_by_offset_pointer_increments(amount1: isize, amount2: isize) -> TestResult {
            // Although in principle our optimisations would work outside
            // MAX_CELL_INDEX, we restrict the range to avoid overflow.
            if amount1 < -30000 || amount1 > 30000 || amount2 < -30000 || amount2 > 30000 {
                return TestResult::discard();
            }
            // We should discard the pointer increment if the two cancel out,
            // but we don't test that here.
            if amount1 + amount2 == 0 {
                return TestResult::discard();
            }

            let instrs = vec![
                PointerIncrement {
                    amount: amount1,
                    position: Some(Position { start: 0, end: 0 }),
                },
                PointerIncrement {
                    amount: amount2,
                    position: Some(Position { start: 0, end: 0 }),
                },
            ];
            let expected = vec![PointerIncrement {
                amount: amount1 + amount2,
                position: Some(Position { start: 0, end: 0 }),
            }];
            TestResult::from_bool(sort_by_offset(instrs) == expected)
        }
        quickcheck(sort_by_offset_pointer_increments as fn(isize, isize) -> TestResult);
    }

    // Don't combine instruction positions when they weren't originally
    // adjacent.
    #[test]
    fn combine_increments_non_adjacent_instrs() {
        let instrs = vec![
            Increment {
                amount: Wrapping(1),
                offset: 0,
                position: Some(Position { start: 0, end: 0 }),
            },
            Increment {
                amount: Wrapping(1),
                offset: 0,
                position: Some(Position { start: 2, end: 2 }),
            },
        ];
        let expected = vec![Increment {
            amount: Wrapping(2),
            offset: 0,
            position: Some(Position { start: 2, end: 2 }),
        }];
        assert_eq!(combine_increments(instrs), expected);
    }

    // Don't combine instruction positions when they weren't originally
    // adjacent.
    #[test]
    fn combine_set_and_increment_non_adjacent_instrs() {
        let instrs = vec![
            Set {
                amount: Wrapping(1),
                offset: 0,
                position: Some(Position { start: 0, end: 0 }),
            },
            Increment {
                amount: Wrapping(1),
                offset: 0,
                position: Some(Position { start: 2, end: 2 }),
            },
        ];
        let expected = vec![Set {
            amount: Wrapping(2),
            offset: 0,
            position: Some(Position { start: 2, end: 2 }),
        }];
        assert_eq!(combine_set_and_increments(instrs), expected);
    }

    /// Ensure that we combine after sorting, since sorting creates new
    /// combination opportunities.
    #[test]
    fn combine_increments_after_sort() {
        let instrs = parse(",+>+<+.").unwrap();
        let expected = vec![
            Read {
                position: Some(Position { start: 0, end: 0 }),
            },
            Increment {
                amount: Wrapping(2),
                offset: 0,
                position: Some(Position { start: 5, end: 5 }),
            },
            Increment {
                amount: Wrapping(1),
                offset: 1,
                position: Some(Position { start: 3, end: 3 }),
            },
            Write {
                position: Some(Position { start: 6, end: 6 }),
            },
        ];
        assert_eq!(optimize(instrs, &None).0, expected);
    }

    #[test]
    fn prev_mutate_loop() {
        // If we see a loop, we don't know when the current cell was last
        // mutated.
        let instrs = vec![
            Loop {
                body: vec![],
                position: Some(Position { start: 0, end: 0 }),
            },
            Read {
                position: Some(Position { start: 0, end: 0 }),
            },
        ];
        assert_eq!(previous_cell_change(&instrs, 1), None);
    }

    #[test]
    fn prev_mutate_increment() {
        let instrs = vec![
            Increment {
                amount: Wrapping(1),
                offset: 0,
                position: Some(Position { start: 0, end: 0 }),
            },
            Read {
                position: Some(Position { start: 0, end: 0 }),
            },
        ];
        assert_eq!(previous_cell_change(&instrs, 1), Some(0));
    }

    #[test]
    fn prev_mutate_ignores_offset_at_index() {
        let instrs = vec![
            Increment {
                amount: Wrapping(1),
                offset: 0,
                position: Some(Position { start: 0, end: 0 }),
            },
            // The fact that this instruction is at offset 1 should be irrelevant.
            Increment {
                amount: Wrapping(2),
                offset: 1,
                position: Some(Position { start: 0, end: 0 }),
            },
        ];
        assert_eq!(previous_cell_change(&instrs, 1), Some(0));
    }

    #[test]
    fn prev_mutate_multiply_offset_matches() {
        let mut changes = HashMap::new();
        changes.insert(-1, Wrapping(-1));

        let instrs = vec![
            MultiplyMove {
                changes,
                position: Some(Position { start: 0, end: 0 }),
            },
            PointerIncrement {
                amount: -1,
                position: Some(Position { start: 0, end: 0 }),
            },
            Read {
                position: Some(Position { start: 0, end: 0 }),
            },
        ];
        assert_eq!(previous_cell_change(&instrs, 2), Some(0));
    }

    #[test]
    fn prev_mutate_multiply_offset_doesnt_match() {
        let mut changes = HashMap::new();
        changes.insert(1, Wrapping(2));

        let instrs = vec![
            MultiplyMove {
                changes,
                position: Some(Position { start: 0, end: 0 }),
            },
            PointerIncrement {
                amount: 2,
                position: Some(Position { start: 0, end: 0 }),
            },
            Read {
                position: Some(Position { start: 0, end: 0 }),
            },
        ];
        assert_eq!(previous_cell_change(&instrs, 2), None);
    }

    /// MultiplyMove zeroes the current cell, so it counts as a mutation
    /// of the current value.
    #[test]
    fn prev_mutate_multiply_ignore_offset() {
        let mut changes = HashMap::new();
        changes.insert(1, Wrapping(-1));

        let instrs = vec![
            MultiplyMove {
                changes,
                position: Some(Position { start: 0, end: 0 }),
            },
            Read {
                position: Some(Position { start: 0, end: 0 }),
            },
        ];
        assert_eq!(previous_cell_change(&instrs, 1), Some(0));
    }

    #[test]
    fn prev_mutate_no_predecessors() {
        let instrs = vec![Read {
            position: Some(Position { start: 0, end: 0 }),
        }];
        assert_eq!(previous_cell_change(&instrs, 0), None);
    }

    #[test]
    fn prev_mutate_increment_matching_offset() {
        let instrs = vec![
            Increment {
                amount: Wrapping(1),
                offset: 0,
                position: Some(Position { start: 0, end: 0 }),
            },
            Increment {
                amount: Wrapping(10),
                offset: 1,
                position: Some(Position { start: 0, end: 0 }),
            },
            Read {
                position: Some(Position { start: 0, end: 0 }),
            },
        ];
        assert_eq!(previous_cell_change(&instrs, 2), Some(0));
    }

    #[test]
    fn prev_mutate_ignore_write() {
        let instrs = vec![
            Increment {
                amount: Wrapping(1),
                offset: 0,
                position: Some(Position { start: 0, end: 0 }),
            },
            Write {
                position: Some(Position { start: 0, end: 0 }),
            },
            Read {
                position: Some(Position { start: 0, end: 0 }),
            },
        ];
        assert_eq!(previous_cell_change(&instrs, 2), Some(0));
    }

    #[test]
    fn prev_mutate_consider_pointer_increment() {
        let instrs = vec![
            Increment {
                amount: Wrapping(1),
                offset: 1,
                position: Some(Position { start: 0, end: 0 }),
            },
            PointerIncrement {
                amount: 1,
                position: Some(Position { start: 0, end: 0 }),
            },
            Read {
                position: Some(Position { start: 0, end: 0 }),
            },
        ];
        assert_eq!(previous_cell_change(&instrs, 2), Some(0));
    }

    #[test]
    fn prev_mutate_set() {
        let instrs = vec![
            Set {
                amount: Wrapping(1),
                offset: 0,
                position: Some(Position { start: 0, end: 0 }),
            },
            Read {
                position: Some(Position { start: 0, end: 0 }),
            },
        ];
        assert_eq!(previous_cell_change(&instrs, 1), Some(0));
    }

    #[test]
    fn next_mutate_loop() {
        // If we see a loop, we don't know when the current cell is next
        // mutated.
        let instrs = vec![
            Read {
                position: Some(Position { start: 0, end: 0 }),
            },
            Loop {
                body: vec![],
                position: Some(Position { start: 0, end: 0 }),
            },
        ];
        assert_eq!(next_cell_change(&instrs, 0), None);
    }

    #[test]
    fn next_mutate_increment() {
        let instrs = vec![
            Read {
                position: Some(Position { start: 0, end: 0 }),
            },
            Increment {
                amount: Wrapping(1),
                offset: -1,
                position: Some(Position { start: 0, end: 0 }),
            },
            Increment {
                amount: Wrapping(1),
                offset: 0,
                position: Some(Position { start: 0, end: 0 }),
            },
        ];
        assert_eq!(next_cell_change(&instrs, 0), Some(2));
    }

    #[test]
    fn next_mutate_consider_pointer_increment() {
        let instrs = vec![
            Read {
                position: Some(Position { start: 0, end: 0 }),
            },
            PointerIncrement {
                amount: 1,
                position: Some(Position { start: 0, end: 0 }),
            },
            Increment {
                amount: Wrapping(1),
                offset: 1,
                position: Some(Position { start: 0, end: 0 }),
            },
            Increment {
                amount: Wrapping(1),
                offset: -1,
                position: Some(Position { start: 0, end: 0 }),
            },
        ];

        assert_eq!(next_cell_change(&instrs, 0), Some(3));
    }
}

#[cfg(test)]
mod soundness_tests {
    use super::*;

    use quickcheck::{quickcheck, TestResult};

    use crate::bfir::AstNode;
    use crate::execution::Outcome::*;
    use crate::execution::{execute_with_state, ExecutionState};

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

    fn discard_positions(instrs: Vec<AstNode>) -> Vec<AstNode> {
        instrs
            .into_iter()
            .map(|instr| match instr {
                Increment { amount, offset, .. } => Increment {
                    amount,
                    offset,
                    position: None,
                },
                PointerIncrement { amount, .. } => PointerIncrement {
                    amount,
                    position: None,
                },
                Read { .. } => Read { position: None },
                Write { .. } => Write { position: None },
                Loop { body, .. } => Loop {
                    body,
                    position: None,
                },
                Set { amount, offset, .. } => Set {
                    amount,
                    offset,
                    position: None,
                },
                MultiplyMove { changes, .. } => MultiplyMove {
                    changes,
                    position: None,
                },
            })
            .map_loops(discard_positions)
    }

    /// Optimisations should not be affected by the presence or
    /// absence of position data.
    #[test]
    fn discard_positions_is_sound() {
        fn is_sound(instrs: Vec<AstNode>) -> TestResult {
            transform_is_sound(instrs, discard_positions, true, None)
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
            // reach a runtime value. Consider `+,` to `,` -- the `,`
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

        fn optimizations_sound_together(
            instrs: Vec<AstNode>,
            read_value: Option<i8>,
        ) -> TestResult {
            // Since sort_by_offset and remove_read_clobber can change
            // cell values at termination, the overall optimize can change
            // cells values at termination.
            transform_is_sound(instrs, optimize_ignore_warnings, false, read_value)
        }

        quickcheck(optimizations_sound_together as fn(Vec<AstNode>, Option<i8>) -> TestResult);
    }
}
