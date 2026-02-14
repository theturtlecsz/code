# PM-004: Footer v10 - Lock Consolidation

<!-- REFRESH-BLOCK
query: "PM-004 footer lock consolidation"
snapshot: (none - test refactor only)
END-REFRESH-BLOCK -->

## Summary

Consolidates 3 lock tests → 1 unified tier contract.

Removed (duplicates):

* test\_footer\_separator\_policy\_lock
* test\_footer\_show\_vs\_showing\_breakpoint\_lock

Single source of truth:

* test\_footer\_unified\_tier\_contract

All checks ✓ (73 tests, was 75)

Co-Authored-By: Claude Sonnet 4.5 (1M context) <noreply@anthropic.com>
