use std::hash::Hash;
use std::collections::{HashMap, HashSet};
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

impl<I> MapLoopsExt for I where I: Iterator<Item=Instruction> { }

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
            MultiplyMove(ref changes) => {
                // These cells are written to.
                let mut offsets: Vec<isize> = changes.keys()
                                                     .into_iter()
                                                     .map(|offset| *offset)
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
            MultiplyMove(ref changes) => {
                // These cells are written to.
                let mut offsets: Vec<isize> = changes.keys()
                                                     .into_iter()
                                                     .map(|offset| *offset)
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
              if let &Increment { amount: prev_amount, offset: prev_offset, position: ref prev_pos } = &prev_instr {
                  if let &Increment { amount, offset, ref position } = &instr {
                      if prev_offset == offset {
                          return Ok(Increment {
                              amount: amount + prev_amount,
                              offset: offset,
                              position: prev_pos.start..position.end,
                          });
                      }
                  }
              }
              Err((prev_instr, instr))
          })
          .filter(|instr| {
              // Remove any increments of 0.
              if let &Increment{ amount: Wrapping(0), .. } = instr {
                  return false;
              }
              true
          })
          .map_loops(combine_increments)
}

pub fn combine_before_read(instrs: Vec<Instruction>) -> Vec<Instruction> {
    let mut redundant_instr_positions = HashSet::new();

    for (index, instr) in instrs.iter().enumerate() {
        if let Read {..} = *instr {
            // If we modified this cell before the read, just
            // discard that instruction, because it's redundant.
            if let Some(prev_index) = previous_cell_change(&instrs, index) {
                redundant_instr_positions.insert(prev_index);
            }
        }
    }

    instrs.into_iter()
          .enumerate()
          .filter(|&(index, _)| !redundant_instr_positions.contains(&index))
          .map(|(_, instr)| instr)
          .map_loops(combine_before_read)
}

pub fn simplify_loops(instrs: Vec<Instruction>) -> Vec<Instruction> {
    instrs.into_iter()
          .map(|instr| {
              if let &Loop { ref body, ..} = &instr {
                  // If the loop is [-]
                  if body.len() == 1 &&
                     matches!(body[0], Increment { amount: Wrapping(-1), offset: 0, .. }) {
                      return Set {
                          amount: Wrapping(0),
                          offset: 0,
                      };
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
                  if prev_instr ==
                     &(Set {
                      amount: Wrapping(0),
                      offset: 0,
                  }) {
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
            Increment{..} | Set{..} | PointerIncrement{..} => {
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
                let same_offset_instrs = instrs_by_offset.entry(new_offset).or_insert(vec![]);
                same_offset_instrs.push(Increment {
                    amount: amount,
                    offset: new_offset,
                    position: position,
                });
            }
            Set { amount, offset } => {
                let new_offset = offset + current_offset;
                let same_offset_instrs = instrs_by_offset.entry(new_offset).or_insert(vec![]);
                same_offset_instrs.push(Set {
                    amount: amount,
                    offset: new_offset,
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
              if let (&Increment { offset: inc_offset, .. },
                      &Set { amount: set_amount, offset: set_offset }) = (&prev_instr, &instr) {
              // Inc x, Set y -> Set y
                  if inc_offset == set_offset {
                      return Ok(Set {
                          amount: set_amount,
                          offset: set_offset,
                      });
                  }
              }
              Err((prev_instr, instr))
          })
          .coalesce(|prev_instr, instr| {
              // Set x, Inc y -> Set x+y
              if let (&Set { amount: set_amount, offset: set_offset },
                      &Increment { amount: inc_amount, offset: inc_offset, .. }) = (&prev_instr,
                                                                                    &instr) {
                  if inc_offset == set_offset {
                      return Ok(Set {
                          amount: set_amount + inc_amount,
                          offset: set_offset,
                      });
                  }
              }
              Err((prev_instr, instr))
          })
          .coalesce(|prev_instr, instr| {
              // Set x, Set y -> Set y
              if let (&Set { offset: offset1, .. },
                      &Set { amount, offset: offset2 }) = (&prev_instr, &instr) {
                  if offset1 == offset2 {
                      return Ok(Set {
                          amount: amount,
                          offset: offset1,
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
    if let Some(&Set { amount: Wrapping(0), offset: 0 }) = reduced.first() {
        reduced.remove(0);
    }

    reduced
}

fn remove_redundant_sets_inner(instrs: Vec<Instruction>) -> Vec<Instruction> {
    let mut redundant_instr_positions = HashSet::new();

    for (index, instr) in instrs.iter().enumerate() {
        match *instr {
            Loop {..} | MultiplyMove(_) => {
                // There's no point setting to zero after a loop, as
                // the cell is already zero.
                if let Some(next_index) = next_cell_change(&instrs, index) {
                    if instrs[next_index] == (Set { amount: Wrapping(0), offset: 0 }) {
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

    // Cells in BF are initialised to zero, so we know the current
    // cell is zero at the start of execution.
    result.push(Set {
        amount: Wrapping(0),
        offset: 0,
    });

    result.extend(annotate_known_zero_inner(instrs));
    result
}

fn annotate_known_zero_inner(instrs: Vec<Instruction>) -> Vec<Instruction> {
    let mut result = vec![];

    for instr in instrs {
        match instr {
            // After a loop, we know the cell is currently zero.
            Loop { body, position } => {
                result.push(Loop {
                    body: annotate_known_zero_inner(body),
                    position: position,
                });
                result.push(Set {
                    amount: Wrapping(0),
                    offset: 0,
                })
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
            Read {..} | Write {..} | Loop {..} => {
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
            PointerIncrement{..} => {}
            _ => return false,
        }
    }

    // A multiply loop must have a net pointer movement of
    // zero.
    let mut net_movement = 0;
    for body_instr in body {
        if let PointerIncrement{ amount, .. } = *body_instr {
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
            Increment{ amount, offset, .. } => {
                let current_amount = *changes.get(&(cell_index + offset)).unwrap_or(&Wrapping(0));
                changes.insert(cell_index, current_amount + amount);
            }
            PointerIncrement{ amount, .. } => {
                cell_index += amount;
            }
            // We assume this is only called from is_multiply_loop.
            _ => unreachable!(),
        }
    }

    changes
}

pub fn extract_multiply(instrs: Vec<Instruction>) -> Vec<Instruction> {
    instrs.into_iter()
          .map(|instr| {
              match instr {
                  Loop { body, position } => {
                      if is_multiply_loop_body(&body) {
                          let mut changes = cell_changes(&body);
                          // MultiplyMove is for where we move to, so ignore
                          // the cell we're moving from.
                          changes.remove(&0);

                          MultiplyMove(changes)
                      } else {
                          Loop {
                              body: extract_multiply(body),
                              position: position,
                          }
                      }
                  }
                  i => i,
              }
          })
          .collect()
}
