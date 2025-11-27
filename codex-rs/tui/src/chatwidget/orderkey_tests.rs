//! ChatWidget OrderKey generation tests
//!
//! This module contains tests for the OrderKey generation system which prevents
//! message interleaving in the TUI. Tests verify key monotonicity, ordering properties,
//! and lexicographic correctness.
//!
//! Extracted from mod.rs for better maintainability.

use super::*;
use crate::chatwidget::test_support::create_test_widget_for_keygen;
use proptest::prelude::*;

// ===================================================================
// KEY GENERATION TESTS - Task 1: Automated Testing Infrastructure
// ===================================================================

#[tokio::test]
async fn test_next_internal_key_monotonic() {
    // Test that next_internal_key() produces monotonically increasing sequence numbers
    let mut widget = create_test_widget_for_keygen();

    let key1 = widget.next_internal_key();
    let key2 = widget.next_internal_key();
    let key3 = widget.next_internal_key();

    // Sequence numbers should increase
    assert!(key1.seq < key2.seq, "internal_key seq should be monotonic");
    assert!(key2.seq < key3.seq, "internal_key seq should be monotonic");

    // All internal keys should use i32::MAX for out
    assert_eq!(
        key1.out,
        i32::MAX,
        "internal_key should use i32::MAX for out"
    );
    assert_eq!(
        key2.out,
        i32::MAX,
        "internal_key should use i32::MAX for out"
    );
    assert_eq!(
        key3.out,
        i32::MAX,
        "internal_key should use i32::MAX for out"
    );

    // Keys should be properly ordered
    assert!(key1 < key2);
    assert!(key2 < key3);
}

#[tokio::test]
async fn test_next_req_key_top_monotonic() {
    // Test that next_req_key_top() produces monotonically increasing keys
    let mut widget = create_test_widget_for_keygen();

    let key1 = widget.next_req_key_top();
    let key2 = widget.next_req_key_top();
    let key3 = widget.next_req_key_top();

    // Sequence numbers should increase
    assert!(key1.seq < key2.seq, "req_key_top seq should be monotonic");
    assert!(key2.seq < key3.seq, "req_key_top seq should be monotonic");

    // All should use i32::MIN for out (banners at top)
    assert_eq!(key1.out, i32::MIN, "req_key_top should use i32::MIN");
    assert_eq!(key2.out, i32::MIN, "req_key_top should use i32::MIN");
    assert_eq!(key3.out, i32::MIN, "req_key_top should use i32::MIN");

    // Keys should be properly ordered
    assert!(key1 < key2);
    assert!(key2 < key3);
}

#[tokio::test]
async fn test_next_req_key_prompt_monotonic() {
    // Test that next_req_key_prompt() produces monotonically increasing keys
    let mut widget = create_test_widget_for_keygen();

    let key1 = widget.next_req_key_prompt();
    let key2 = widget.next_req_key_prompt();
    let key3 = widget.next_req_key_prompt();

    // Sequence numbers should increase
    assert!(
        key1.seq < key2.seq,
        "req_key_prompt seq should be monotonic"
    );
    assert!(
        key2.seq < key3.seq,
        "req_key_prompt seq should be monotonic"
    );

    // All should use i32::MIN + 1 for out (prompts after banners)
    assert_eq!(
        key1.out,
        i32::MIN + 1,
        "req_key_prompt should use i32::MIN + 1"
    );
    assert_eq!(
        key2.out,
        i32::MIN + 1,
        "req_key_prompt should use i32::MIN + 1"
    );
    assert_eq!(
        key3.out,
        i32::MIN + 1,
        "req_key_prompt should use i32::MIN + 1"
    );

    // Keys should be properly ordered
    assert!(key1 < key2);
    assert!(key2 < key3);
}

#[tokio::test]
async fn test_next_req_key_after_prompt_monotonic() {
    // Test that next_req_key_after_prompt() produces monotonically increasing keys
    let mut widget = create_test_widget_for_keygen();

    let key1 = widget.next_req_key_after_prompt();
    let key2 = widget.next_req_key_after_prompt();
    let key3 = widget.next_req_key_after_prompt();

    // Sequence numbers should increase
    assert!(
        key1.seq < key2.seq,
        "req_key_after_prompt seq should be monotonic"
    );
    assert!(
        key2.seq < key3.seq,
        "req_key_after_prompt seq should be monotonic"
    );

    // All should use i32::MIN + 2 for out (notices after prompts)
    assert_eq!(
        key1.out,
        i32::MIN + 2,
        "req_key_after_prompt should use i32::MIN + 2"
    );
    assert_eq!(
        key2.out,
        i32::MIN + 2,
        "req_key_after_prompt should use i32::MIN + 2"
    );
    assert_eq!(
        key3.out,
        i32::MIN + 2,
        "req_key_after_prompt should use i32::MIN + 2"
    );

    // Keys should be properly ordered
    assert!(key1 < key2);
    assert!(key2 < key3);
}

#[tokio::test]
async fn test_no_collisions_across_key_categories() {
    // Test that interleaved calls to different key generation functions
    // never produce duplicate OrderKey values
    let mut widget = create_test_widget_for_keygen();

    let mut keys = Vec::new();

    // Interleave calls to all four key generation functions
    keys.push(widget.next_internal_key());
    keys.push(widget.next_req_key_top());
    keys.push(widget.next_req_key_prompt());
    keys.push(widget.next_req_key_after_prompt());
    keys.push(widget.next_internal_key());
    keys.push(widget.next_req_key_top());
    keys.push(widget.next_req_key_prompt());
    keys.push(widget.next_req_key_after_prompt());
    keys.push(widget.next_internal_key());
    keys.push(widget.next_req_key_top());

    // Check that all keys are unique
    for i in 0..keys.len() {
        for j in (i + 1)..keys.len() {
            assert_ne!(
                keys[i], keys[j],
                "Keys at positions {} and {} should be unique: {:?} vs {:?}",
                i, j, keys[i], keys[j]
            );
        }
    }
}

#[tokio::test]
async fn test_key_ordering_within_request() {
    // Test key ordering with SPEC-955 Session 2 semantics:
    // - next_req_key_top() increments current_request_index (for banners/status)
    // - next_req_key_prompt() increments current_request_index (for user prompts)
    // - next_req_key_after_prompt() uses current request (for notices after prompt)
    // - next_internal_key() uses last_seen_request_index (for model output)
    //
    // Each call to top/prompt creates a NEW synthetic request to prevent interleaving.
    let mut widget = create_test_widget_for_keygen();

    // Simulate provider being at request 5
    widget.last_seen_request_index = 5;

    let banner = widget.next_req_key_top();
    let prompt = widget.next_req_key_prompt();
    let after_prompt = widget.next_req_key_after_prompt();
    let internal = widget.next_internal_key();

    // Verify ordering: all keys should be monotonically increasing
    assert!(banner < prompt, "banner should come before prompt");
    assert!(
        prompt < after_prompt,
        "prompt should come before after_prompt"
    );
    // Internal key anchors to last_seen (5), which is less than synthetic keys (6, 7, 7)
    // So internal actually comes BEFORE the synthetic keys in sort order
    assert!(internal < banner, "internal (anchored to last_seen=5) < banner (synthetic=6)");

    // Verify the out values are as expected
    assert_eq!(banner.out, i32::MIN);
    assert_eq!(prompt.out, i32::MIN + 1);
    assert_eq!(after_prompt.out, i32::MIN + 2);
    assert_eq!(internal.out, i32::MAX);

    // SPEC-955 Session 2: Each synthetic key type increments current_request_index
    // banner uses max(current+1, last_seen+1) = max(1, 6) = 6
    assert_eq!(banner.req, 6, "banner gets first synthetic req");
    // prompt increments again: max(7, 6) = 7
    assert_eq!(prompt.req, 7, "prompt gets second synthetic req");
    // after_prompt does NOT increment, uses current (7)
    assert_eq!(after_prompt.req, 7, "after_prompt shares req with prompt");
    // internal uses last_seen_request_index directly (5)
    assert_eq!(internal.req, 5, "internal anchors to provider's last_seen");
}

#[tokio::test]
async fn test_key_ordering_across_multiple_requests() {
    // Test key ordering with SPEC-955 Session 2 semantics across provider requests.
    //
    // Key insight: There are TWO request numbering systems:
    // 1. last_seen_request_index: The provider's actual request counter (from API)
    // 2. current_request_index: Synthetic counter for user-side content
    //
    // Internal keys anchor to last_seen (provider), synthetic keys use current (user).
    let mut widget = create_test_widget_for_keygen();

    // Provider request 1
    widget.last_seen_request_index = 1;
    let req1_banner = widget.next_req_key_top();
    let req1_prompt = widget.next_req_key_prompt();
    let req1_internal = widget.next_internal_key();

    // Provider request 2
    widget.last_seen_request_index = 2;
    let req2_banner = widget.next_req_key_top();
    let req2_prompt = widget.next_req_key_prompt();
    let req2_internal = widget.next_internal_key();

    // Verify internal keys (model output) sort by provider's last_seen
    assert!(
        req1_internal < req2_internal,
        "Internal keys should sort by provider request order"
    );
    assert_eq!(req1_internal.req, 1, "internal1 anchored to last_seen=1");
    assert_eq!(req2_internal.req, 2, "internal2 anchored to last_seen=2");

    // Verify synthetic keys (banners, prompts) sort by their generation order
    assert!(req1_banner < req1_prompt, "banner1 < prompt1 (sequential generation)");
    assert!(req1_prompt < req2_banner, "prompt1 < banner2 (sequential generation)");
    assert!(req2_banner < req2_prompt, "banner2 < prompt2 (sequential generation)");

    // Internal keys anchor to last_seen, which is typically LESS than synthetic keys
    // (synthetic keys advance current_request_index beyond last_seen+1)
    // So internal1 (req=1) < banner1 (req=2)
    assert!(
        req1_internal < req1_banner,
        "Internal (req=1) < synthetic banner (req=2)"
    );

    // Verify the actual req values match SPEC-955 Session 2 behavior
    assert_eq!(req1_banner.req, 2, "banner1: max(1, 1+1) = 2");
    assert_eq!(req1_prompt.req, 3, "prompt1: max(3, 1+1) = 3");
    assert_eq!(req2_banner.req, 4, "banner2: max(4, 2+1) = 4");
    assert_eq!(req2_prompt.req, 5, "prompt2: max(5, 2+1) = 5");
}

#[test]
fn test_orderkey_lexicographic_ordering() {
    // Test that OrderKey::cmp implements correct lexicographic ordering
    // (req, out, seq) where req is primary, out is secondary, seq is tertiary

    let key1 = OrderKey {
        req: 1,
        out: 0,
        seq: 0,
    };
    let key2 = OrderKey {
        req: 2,
        out: -100,
        seq: 0,
    };
    assert!(key1 < key2, "lower req should always sort first");

    let key3 = OrderKey {
        req: 5,
        out: i32::MIN,
        seq: 0,
    };
    let key4 = OrderKey {
        req: 5,
        out: i32::MAX,
        seq: 0,
    };
    assert!(key3 < key4, "same req, lower out should sort first");

    let key5 = OrderKey {
        req: 7,
        out: 0,
        seq: 1,
    };
    let key6 = OrderKey {
        req: 7,
        out: 0,
        seq: 2,
    };
    assert!(key5 < key6, "same req and out, lower seq should sort first");

    let key7 = OrderKey {
        req: 3,
        out: i32::MAX,
        seq: 999,
    };
    let key8 = OrderKey {
        req: 4,
        out: i32::MIN,
        seq: 0,
    };
    assert!(key7 < key8, "req takes precedence over out and seq");
}

#[tokio::test]
async fn test_internal_seq_increments_globally() {
    // Verify that internal_seq increments across ALL key generation functions,
    // providing a global ordering tie-breaker
    let mut widget = create_test_widget_for_keygen();

    let initial_seq = widget.internal_seq;

    let _k1 = widget.next_internal_key();
    assert_eq!(widget.internal_seq, initial_seq + 1);

    let _k2 = widget.next_req_key_top();
    assert_eq!(widget.internal_seq, initial_seq + 2);

    let _k3 = widget.next_req_key_prompt();
    assert_eq!(widget.internal_seq, initial_seq + 3);

    let _k4 = widget.next_req_key_after_prompt();
    assert_eq!(widget.internal_seq, initial_seq + 4);

    let _k5 = widget.next_internal_key();
    assert_eq!(widget.internal_seq, initial_seq + 5);
}

#[tokio::test]
async fn test_key_generation_with_pending_user_prompts() {
    // Test that next_internal_key() correctly accounts for pending_user_prompts_for_next_turn
    let mut widget = create_test_widget_for_keygen();

    widget.last_seen_request_index = 10;
    widget.pending_user_prompts_for_next_turn = 1;

    // Internal key should use req = 11 (next turn) when pending prompts exist
    let internal = widget.next_internal_key();
    assert_eq!(
        internal.req, 11,
        "internal_key should advance to next req when pending prompts"
    );

    // Verify current_request_index was updated
    assert!(widget.current_request_index >= 11);
}

// ===================================================================
// PROPERTY-BASED TESTS - Advanced Validation with Random Inputs
// ===================================================================

// Strategy: Generate arbitrary OrderKeys
fn arbitrary_orderkey() -> impl Strategy<Value = OrderKey> {
    (any::<u64>(), any::<i32>(), any::<u64>()).prop_map(|(req, out, seq)| OrderKey {
        req,
        out,
        seq,
    })
}

proptest! {
    #[test]
    fn prop_orderkey_transitivity(keys in prop::collection::vec(arbitrary_orderkey(), 3..10)) {
        // Property: OrderKey ordering is transitive (A < B && B < C => A < C)
        for i in 0..keys.len() {
            for j in 0..keys.len() {
                for k in 0..keys.len() {
                    if keys[i] < keys[j] && keys[j] < keys[k] {
                        prop_assert!(
                            keys[i] < keys[k],
                            "Transitivity violated at indices {},{},{}",
                            i, j, k
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn prop_orderkey_req_dominates(
        req1 in any::<u64>(),
        req2 in any::<u64>(),
        out1 in any::<i32>(),
        out2 in any::<i32>(),
        seq1 in any::<u64>(),
        seq2 in any::<u64>()
    ) {
        // Property: req field always dominates (regardless of out/seq values)
        if req1 < req2 {
            let key1 = OrderKey { req: req1, out: out1, seq: seq1 };
            let key2 = OrderKey { req: req2, out: out2, seq: seq2 };
            prop_assert!(key1 < key2, "Lower req must sort first");
        }
    }

    #[test]
    fn prop_orderkey_groups_by_request(
        keys in prop::collection::vec(arbitrary_orderkey(), 10..30)
    ) {
        // Property: When sorted, all keys with same req are contiguous
        let mut sorted = keys.clone();
        sorted.sort();

        // Verify grouping by tracking req transitions
        let mut last_req: Option<u64> = None;
        let mut seen_reqs = std::collections::HashSet::new();

        for key in sorted {
            if let Some(prev) = last_req
                && key.req != prev {
                    // Transitioning to new req - should never see it again
                    prop_assert!(
                        !seen_reqs.contains(&key.req),
                        "req {} appeared non-contiguously", key.req
                    );
                    seen_reqs.insert(prev);
                }
            last_req = Some(key.req);
        }
    }

    #[test]
    fn prop_orderkey_deterministic_sorting(
        mut keys in prop::collection::vec(arbitrary_orderkey(), 10..50)
    ) {
        // Property: Sorting is deterministic and stable
        let original = keys.clone();

        keys.sort();
        let first_sort = keys.clone();

        keys.sort();
        let second_sort = keys;

        prop_assert_eq!(first_sort.len(), original.len(), "No keys lost");
        prop_assert_eq!(first_sort, second_sort, "Sorting must be deterministic");
    }
}
