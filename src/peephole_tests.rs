use std::collections::HashMap;
use std::num::Wrapping;

use pretty_assertions::assert_eq;
use quickcheck::quickcheck;

use crate::bfir::AstNode::*;
use crate::bfir::{AstNode, Position};
use crate::diagnostics::Warning;

use crate::bfir::parse;
use crate::peephole::*;
use quickcheck::{Arbitrary, Gen, TestResult};

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
        matches!(annotated[0], Set { amount: Wrapping(0), offset: 0, .. })
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
