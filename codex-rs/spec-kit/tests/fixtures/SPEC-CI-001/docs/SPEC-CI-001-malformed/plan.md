# SPEC-CI-001-malformed: Malformed JSON Case

## Objective
Test case with invalid JSON in consensus file.

## Expected Behavior
- Default: Exit code 0, advisory signal
- --strict-schema: Exit code 3, error
