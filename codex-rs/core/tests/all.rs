// Single integration test binary that aggregates all test modules.
// The submodules live in `tests/all/`.

// SPEC-957: Allow dead code in test modules - some tests are stubbed/ignored
#![allow(dead_code)]
// SPEC-957: Allow expect in tests
#![allow(clippy::expect_used)]

mod suite;
