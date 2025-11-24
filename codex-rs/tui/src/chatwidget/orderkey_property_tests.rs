use super::OrderKey;
use proptest::prelude::*;

// Strategy for generating arbitrary OrderKeys
prop_compose! {
    fn arbitrary_orderkey()
        (req in 1u64..100, out in -10i32..10, seq in 0u64..1000)
        -> OrderKey
    {
        OrderKey { req, out, seq }
    }
}

proptest! {
    #[test]
    fn prop_orderkey_transitivity(
        keys in prop::collection::vec(arbitrary_orderkey(), 3..20)
    ) {
        // Property: If a < b and b < c, then a < c
        for i in 0..keys.len() {
            for j in i+1..keys.len() {
                for k in j+1..keys.len() {
                    if keys[i] < keys[j] && keys[j] < keys[k] {
                        prop_assert!(keys[i] < keys[k],
                            "Transitivity violated: {:?} < {:?} < {:?} but {:?} >= {:?}",
                            keys[i], keys[j], keys[k], keys[i], keys[k]);
                    }
                }
            }
        }
    }

    #[test]
    fn prop_orderkey_totality(
        a in arbitrary_orderkey(),
        b in arbitrary_orderkey()
    ) {
        // Property: For any two keys, exactly one of <, >, or == holds
        let cmp = a.cmp(&b);
        prop_assert!(
            matches!(cmp, std::cmp::Ordering::Less | std::cmp::Ordering::Equal | std::cmp::Ordering::Greater),
            "Comparison must yield a valid ordering"
        );
    }

    #[test]
    fn prop_orderkey_request_dominance(
        req1 in 1u64..100,
        req2 in 1u64..100,
        out1 in -10i32..10,
        out2 in -10i32..10,
        seq1 in 0u64..1000,
        seq2 in 0u64..1000
    ) {
        // Property: Request ordinal is primary sort key
        // If req1 < req2, then OrderKey(req1, *, *) < OrderKey(req2, *, *)
        if req1 < req2 {
            let key1 = OrderKey { req: req1, out: out1, seq: seq1 };
            let key2 = OrderKey { req: req2, out: out2, seq: seq2 };
            prop_assert!(key1 < key2,
                "Request {} should come before request {}, but {:?} >= {:?}",
                req1, req2, key1, key2);
        }
    }

    #[test]
    fn prop_orderkey_sorting_stable(
        mut keys in prop::collection::vec(arbitrary_orderkey(), 5..50)
    ) {
        // Property: Sorting is deterministic and stable
        let original_len = keys.len();
        keys.sort();

        // Sort again
        let first_sort = keys.clone();
        keys.sort();

        // Should be identical
        prop_assert_eq!(&first_sort, &keys, "Sorting should be deterministic");

        // All elements preserved
        prop_assert_eq!(original_len, keys.len(), "Sorting should preserve all elements");
    }
}
