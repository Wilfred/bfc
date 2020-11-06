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

In addition to conventional unit tests, bfc uses quickcheck

bfc uses quickcheck to ensure optimisations are in the optimal order
(by verifying that our optimiser is idempotent).

## Integration Tests

```
$ ./integration_tests.sh
```

This script compiles real BF programs, runs them, and verifies their
output matches the corresponding `.out` file.
