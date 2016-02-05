//! Optimisations that replace parts of the BF AST with faster
//! equivalents.

use std::hash::Hash;
use std::collections::{HashMap, HashSet};
use std::num::Wrapping;

use itertools::Itertools;

use diagnostics::Warning;

use bfir::{Instruction, Position, Combine, get_position};
use bfir::Instruction::*;

const MAX_OPT_ITERATIONS: u64 = 40;

/// Given a sequence of BF instructions, apply peephole optimisations
/// (repeatedly if necessary).
pub fn optimize(instrs: Vec<Instruction>,
                pass_specification: &Option<String>)
                -> (Vec<Instruction>, Vec<Warning>) {
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
    println!("Warning: ran peephole optimisations {} times but did not reach a fixed point!",
             MAX_OPT_ITERATIONS);

    (result, warnings)
}

/// Apply all our peephole optimisations once and return the result.
fn optimize_once(instrs: Vec<Instruction>,
                 pass_specification: &Option<String>)
                 -> (Vec<Instruction>, Option<Warning>) {
    let pass_specification = pass_specification.clone()
                                               .unwrap_or("combine_inc,combine_ptr,known_zero,\
                                                           zeroing_loop,combine_set,\
                                                           dead_loop,redundant_set,read_clobber,\
                                                           pure_removal,offset_sort"
                                                              .to_owned());
    let passes: Vec<_> = pass_specification.split(",").collect();

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
    if passes.contains(&"zeroing_loop") {
        instrs = simplify_loops(instrs);
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
        instrs = combine_before_read(instrs);
    }
    let warning;
    if passes.contains(&"pure_removal") {
        let (removed, pure_warning) = remove_pure_code(instrs);
        instrs = removed;
        warning = pure_warning;
    } else {
        warning = None;
    }

    if passes.contains(&"offset_sort") {
        instrs = sort_by_offset(instrs);
    }

    (instrs, warning)
}

/// Defines a method on iterators to map a function over all loop bodies.
trait MapLoopsExt: Iterator<Item=Instruction> {
    fn map_loops<F>(&mut self, f: F) -> Vec<Instruction>
        where F: Fn(Vec<Instruction>) -> Vec<Instruction>
    {
        self.map(|instr| {
                match instr {
                    Loop { body, position } => {
                        Loop {
                            body: f(body),
                            position: position,
                        }
                    }
                    other => other,
                }
            })
            .collect()
    }
}

impl<I> MapLoopsExt for I where I: Iterator<Item = Instruction>
{}

/// Given an index into a vector of instructions, find the index of
/// the previous instruction that modified the current cell. If we're
/// unsure, or there isn't one, return None.
///
/// Note this totally ignores the instruction at the index given, even
/// if it has an offset. E.g. if the instruction is
/// Set {amount:100, offset: 1}, we're still considering previous instructions that
/// modify the current cell, not the (cell_index + 1)th cell.
pub fn previous_cell_change(instrs: &[Instruction], index: usize) -> Option<usize> {
    assert!(index < instrs.len());

    let mut needed_offset = 0;
    for i in (0..index).rev() {
        match instrs[i] {
            Increment { offset, .. } => {
                if offset == needed_offset {
                    return Some(i);
                }
            }
            Set { offset, .. } => {
                if offset == needed_offset {
                    return Some(i);
                }
            }
            PointerIncrement { amount, .. } => {
                needed_offset += amount;
            }
            MultiplyMove { ref changes, .. } => {
                // These cells are written to.
                let mut offsets: Vec<isize> = changes.keys()
                                                     .into_iter()
                                                     .cloned()
                                                     .collect();
                // This cell is zeroed.
                offsets.push(0);

                if offsets.contains(&needed_offset) {
                    return Some(i);
                }
            }
            // No cells changed, so just keep working backwards.
            Write {..} => {}
            // These instructions may have modified the cell, so
            // we return None for "I don't know".
            Read {..} | Loop {..} => return None,
        }
    }
    None
}

/// Inverse of previous_cell_change.
///
/// This is very similar to previous_cell_change and previous
/// implementations called previous_cell_change on the reversed
/// vector. This proved extremely hard to reason about. Instead, we
/// have copied the body of previous_cell_change and highlighted the
/// differences.
pub fn next_cell_change(instrs: &[Instruction], index: usize) -> Option<usize> {
    assert!(index < instrs.len());

    let mut needed_offset = 0;
    // Unlike previous_cell_change, we iterate forward.
    for i in (index + 1)..instrs.len() {
        match instrs[i] {
            Increment { offset, .. } => {
                if offset == needed_offset {
                    return Some(i);
                }
            }
            Set { offset, .. } => {
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
                let mut offsets: Vec<isize> = changes.keys()
                                                     .into_iter()
                                                     .cloned()
                                                     .collect();
                // This cell is zeroed.
                offsets.push(0);

                if offsets.contains(&needed_offset) {
                    return Some(i);
                }
            }
            // No cells changed, so just keep working backwards.
            Write {..} => {}
            // These instructions may have modified the cell, so
            // we return None for "I don't know".
            Read {..} | Loop {..} => return None,
        }
    }
    None
}

/// Combine consecutive increments into a single increment
/// instruction.
pub fn combine_increments(instrs: Vec<Instruction>) -> Vec<Instruction> {
    instrs.into_iter()
          .coalesce(|prev_instr, instr| {
              // Collapse consecutive increments.
              if let Increment { amount: prev_amount, offset: prev_offset, position: prev_pos } =
                     prev_instr {
                  if let Increment { amount, offset, position } = instr {
                      if prev_offset == offset {
                          return Ok(Increment {
                              amount: amount + prev_amount,
                              offset: offset,
                              position: prev_pos.combine(position),
                          });
                      }
                  }
              }
              Err((prev_instr, instr))
          })
          .filter(|instr| {
              // Remove any increments of 0.
              if let Increment { amount: Wrapping(0), .. } = *instr {
                  return false;
              }
              true
          })
          .map_loops(combine_increments)
}

pub fn combine_ptr_increments(instrs: Vec<Instruction>) -> Vec<Instruction> {
    instrs.into_iter()
          .coalesce(|prev_instr, instr| {
              // Collapse consecutive increments.
              if let PointerIncrement { amount: prev_amount, position: prev_pos } = prev_instr {
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

// TODO: rename, this isn't really a combine, this really a dead code
// removal.
// TODO: this should generate a warning too.
pub fn combine_before_read(instrs: Vec<Instruction>) -> Vec<Instruction> {
    let mut redundant_instr_positions = HashSet::new();
    let mut last_write_index = None;

    for (index, instr) in instrs.iter().enumerate() {
        match *instr {
            Read {..} => {
                // If we can find the time this cell was modified:
                if let Some(prev_modify_index) = previous_cell_change(&instrs, index) {

                    let mut write_after_modify = false;
                    if let Some(write_index) = last_write_index {
                        if write_index > prev_modify_index {
                            write_after_modify = true;
                        }
                    }

                    if !write_after_modify {
                        redundant_instr_positions.insert(prev_modify_index);
                    }
                }
            }
            Write {..} => {
                last_write_index = Some(index);
            }
            _ => {}
        }
    }

    instrs.into_iter()
          .enumerate()
          .filter(|&(index, _)| !redundant_instr_positions.contains(&index))
          .map(|(_, instr)| instr)
          .map_loops(combine_before_read)
}

// TODO: rename this to zeroing_loop.
pub fn simplify_loops(instrs: Vec<Instruction>) -> Vec<Instruction> {
    instrs.into_iter()
          .map(|instr| {
              if let Loop { ref body, position } = instr {
                  // If the loop is [-]
                  if body.len() == 1 {
                      if let Increment { amount: Wrapping(-1), offset: 0, .. } = body[0] {
                          return Set {
                              amount: Wrapping(0),
                              offset: 0,
                              position: position,
                          };
                      }
                  }
              }
              instr
          })
          .map_loops(simplify_loops)
}

/// Remove any loops where we know the current cell is zero.
pub fn remove_dead_loops(instrs: Vec<Instruction>) -> Vec<Instruction> {
    instrs.clone()
          .into_iter()
          .enumerate()
          .filter(|&(index, ref instr)| {
              match *instr {
                  Loop {..} => {}
                  // Keep all instructions that aren't loops.
                  _ => {
                      return true;
                  }
              }

              // Find the previous change instruction:
              if let Some(prev_change_index) = previous_cell_change(&instrs, index) {
                  let prev_instr = &instrs[prev_change_index];
                  // If the previous instruction set to zero, our loop is dead.
                  if let Set { amount: Wrapping(0), offset: 0, .. } = *prev_instr {
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
pub fn sort_by_offset(instrs: Vec<Instruction>) -> Vec<Instruction> {
    let mut sequence = vec![];
    let mut result = vec![];

    for instr in instrs {
        match instr {
            Increment {..} | Set {..} | PointerIncrement {..} => {
                sequence.push(instr);
            }
            _ => {
                if !sequence.is_empty() {
                    result.extend(sort_sequence_by_offset(sequence));
                    sequence = vec![];
                }
                if let Loop { body, position } = instr {
                    result.push(Loop {
                        body: sort_by_offset(body),
                        position: position,
                    });
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
    items.into_iter().map(|(_, v)| v).collect()
}

/// Given a BF program, combine sets/increments using offsets so we
/// have single PointerIncrement at the end.
pub fn sort_sequence_by_offset(instrs: Vec<Instruction>) -> Vec<Instruction> {
    let mut instrs_by_offset: HashMap<isize, Vec<Instruction>> = HashMap::new();
    let mut current_offset = 0;
    let mut last_ptr_inc_pos = None;

    for instr in instrs {
        match instr {
            Increment { amount, offset, position } => {
                let new_offset = offset + current_offset;
                let same_offset_instrs = instrs_by_offset.entry(new_offset).or_insert_with(|| vec![]);
                same_offset_instrs.push(Increment {
                    amount: amount,
                    offset: new_offset,
                    position: position,
                });
            }
            Set { amount, offset, position } => {
                let new_offset = offset + current_offset;
                let same_offset_instrs = instrs_by_offset.entry(new_offset).or_insert_with(|| vec![]);
                same_offset_instrs.push(Set {
                    amount: amount,
                    offset: new_offset,
                    position: position,
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
    let mut results: Vec<Instruction> = vec![];
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
pub fn combine_set_and_increments(instrs: Vec<Instruction>) -> Vec<Instruction> {
    // It's sufficient to consider immediately adjacent instructions
    // as sort_sequence_by_offset ensures that if the offset is the
    // same, the instruction is adjacent.
    instrs.into_iter()
          .coalesce(|prev_instr, instr| {
              // TODO: Set, Write, Increment -> Set, Write, Set
              // Inc x, Set y -> Set y
              if let (&Increment { offset: inc_offset, position: inc_pos, .. },
                      &Set { amount: set_amount, offset: set_offset, position: set_pos }) =
                     (&prev_instr, &instr) {
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
              if let Set { amount: set_amount, offset: set_offset, position: set_pos } =
                     prev_instr {
                  if let Increment { amount: inc_amount, offset: inc_offset, position: inc_pos } =
                         instr {
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
              if let (&Set { offset: offset1, position: position1, .. },
                      &Set { amount, offset: offset2, position: position2 }) = (&prev_instr,
                                                                                &instr) {
                  if offset1 == offset2 {
                      return Ok(Set {
                          amount: amount,
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

pub fn remove_redundant_sets(instrs: Vec<Instruction>) -> Vec<Instruction> {
    let mut reduced = remove_redundant_sets_inner(instrs);

    // Remove a set zero at the beginning of the program, since cells
    // are initialised to zero anyway.
    if let Some(&Set { amount: Wrapping(0), offset: 0, .. }) = reduced.first() {
        reduced.remove(0);
    }

    reduced
}

fn remove_redundant_sets_inner(instrs: Vec<Instruction>) -> Vec<Instruction> {
    let mut redundant_instr_positions = HashSet::new();

    for (index, instr) in instrs.iter().enumerate() {
        match *instr {
            Loop {..} | MultiplyMove {..} => {
                // There's no point setting to zero after a loop, as
                // the cell is already zero.
                if let Some(next_index) = next_cell_change(&instrs, index) {
                    if let Set { amount: Wrapping(0), offset: 0, .. } = instrs[next_index] {
                        redundant_instr_positions.insert(next_index);
                    }
                }
            }
            _ => {}
        }
    }

    instrs.into_iter()
          .enumerate()
          .filter(|&(index, _)| !redundant_instr_positions.contains(&index))
          .map(|(_, instr)| instr)
          .map_loops(remove_redundant_sets_inner)
}

pub fn annotate_known_zero(instrs: Vec<Instruction>) -> Vec<Instruction> {
    let mut result = vec![];

    let position = if instrs.is_empty() {
        None
    } else {
        get_position(&instrs[0]).map(|first_instr_pos| {
            Position {
                start: first_instr_pos.start,
                end: first_instr_pos.start,
            }
        })
    };

    // Cells in BF are initialised to zero, so we know the current
    // cell is zero at the start of execution.
    let set_instr = Set {
        amount: Wrapping(0),
        offset: 0,
        position: position,
    };
    // Insert the set instruction unless there is one already present.
    if instrs.first() != Some(&set_instr) {
        result.push(set_instr);
    }

    result.extend(annotate_known_zero_inner(instrs));
    result
}

fn annotate_known_zero_inner(instrs: Vec<Instruction>) -> Vec<Instruction> {
    let mut result = vec![];

    for (i, item) in instrs.iter().enumerate() {
        let instr = item.clone();

        match instr {
            // After a loop, we know the cell is currently zero.
            Loop { body, position } => {
                result.push(Loop {
                    body: annotate_known_zero_inner(body),
                    position: position,
                });
                // Treat this set as positioned at the ].
                let set_pos = position.map(|loop_pos| {
                    Position {
                        start: loop_pos.end,
                        end: loop_pos.end,
                    }
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
pub fn remove_pure_code(mut instrs: Vec<Instruction>) -> (Vec<Instruction>, Option<Warning>) {
    let mut pure_instrs = vec![];
    while !instrs.is_empty() {
        let last_instr = instrs.pop().unwrap();

        match last_instr {
            Read {..} | Write {..} | Loop {..} => {
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
        let position = pure_instrs.into_iter()
                                  .map(|instr| get_position(&instr))
                                  .filter(|pos| pos.is_some())
                                  .fold1(|pos1, pos2| pos1.combine(pos2))
                                  .unwrap();
        Some(Warning {
            message: "These instructions have no effect.".to_owned(),
            position: position,
        })
    };

    (instrs, warning)
}
