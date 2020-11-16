---
id: testing
title: Testing
---

This page describes how bfc is tested. This is primarily useful for
contributors, but is also interesting for readers working on their own
PL implementations.

## Unit Tests

You can run all bfc's unit tests with Cargo.

```
$ cargo test
```

This runs basic unit tests, property-based tests, and LLVM snapshots
tests.

### Quickcheck

bfc includes property-based tests using the [Rust quickcheck
library](https://github.com/BurntSushi/quickcheck). These operate on
the intermediate representation used in bfc, called BFIR.

These tests verify several properties for optimisations: runtime
improvement, idempotence and behavioural equivalence.

### Verifying Improvement

bfc optimisation should never increase the number of BFIR
instructions.

`f(program)` should have fewer (or the same) number of instructions as
`program`.

### Verifying Idempotence

All bfc optimisation passes are idempotent. Running an
optimisation twice should produce the same output as running it
once. 

This property applies to individual peephole optimisations, as well as
the combined optimiser. Testing this catches phase ordering issues, as
well as optimisations that need to compute a fixpoint.

`f(f(program))` should be the same as `f(program)`.

### Verifying Behavioural Equivalence

bfc opimisation passes should never change the semantics of a program.

`eval(program)` should give the same result as `eval(f(program))`,
provided that `eval(program)` terminates within a specific number of
steps. bfc checks this by evaluating the code within a sandbox.

This same evaluator is also used during compilation, for compile-time
evaluation of code!

If `program` relies on runtime inputs, the test can verify the program
state prior to the first runtime input, or choose an arbitrary input.

Some tests also check the cell state after executing `program`, which
should be the same as executing `f(program)`. This is not true of all
optimisation passes: dead code may modify cells that are unused.

### Finding Interesting Programs

BFIR defines 7 different expressions. Randomly generated IR would only
have a 1/7 chance of producing a loop, so only a 1/49 chance of
producing a nested loop.

Nested loops, particularly including multiply loops, tend to expose
interesting bugs. BFIR generation in tests overweights these kind of
loops, so tests spend more time exercising more complicated loops.

### LLVM Snapshot Tests

The file `llvm_tests.rs` tests that certain BF programs produce the
expected LLVM IR output.

These tests tend to be brittle, especially when upgrading major LLVM
versions. They are useful for verifying that unrelated refactoring
hasn't changed the compiled output.

## Integration Tests

```
$ ./integration_tests.sh
```

This script compiles real BF programs, runs them, and verifies their
output matches the corresponding `.out` file.

This is the final step in bfc testing. It catches issues that only
occur in larger, real-word BF programs.
