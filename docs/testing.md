---
id: testing
title: Testing
---

bfc has an extensive test suite (although bugs still slip through).

## Unit Tests

```
$ cargo test
```

bfc uses quickcheck to ensure optimisations are in the optimal order
(by verifying that our optimiser is idempotent).

## Integration Tests

```
$ ./integration_tests.sh
```
