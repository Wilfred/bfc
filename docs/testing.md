---
id: testing
title: Testing
---

bfc has an extensive test suite (although bugs still slip through).

## Unit Tests

```
$ cargo test
```

### Quickcheck

bfc includes property-based tests using the [Rust quickcheck
library](https://github.com/BurntSushi/quickcheck).

These verify two important properties for optimisations: idempotence
and behavioural equivalence.

### Verifying Idempotence

bfc includes property-based tests using the [Rust quickcheck
library](https://github.com/BurntSushi/quickcheck).

bfc verifies that optimisation passes are idempotent. Running an
optimisation twice should produce the same output as running it
once. `f(f(program))` should be the same as `f(program)`.

This applies to individual peephole optimisations, as well as the
combined optimiser. This catches phase ordering issues, as well as
cases needing a fixpoint.

### Verifying Behavioural Equivalence

`eval(program)` should give the same result as `eval(f(program))`.

### Finding Interesting Programs

Fuzzing is tuned to increase the likelihood of nested loops. Too
unlikely with a naive implementation.

## Integration Tests

```
$ ./integration_tests.sh
```

This script compiles real BF programs, runs them, and verifies their
output matches the corresponding `.out` file.
