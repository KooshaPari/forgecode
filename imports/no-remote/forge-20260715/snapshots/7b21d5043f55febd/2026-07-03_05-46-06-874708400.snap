#![no_main]
use libfuzzer_sys::fuzz_target;
use focus_ir::RuleIr;
use sha2::{Digest, Sha256};

/// Generate canonical JSON hash for RuleIr.
fn canonical_hash(rule: &RuleIr) -> String {
    let json = serde_json::to_string(&rule).expect("serialize RuleIr");
    let mut hasher = Sha256::new();
    hasher.update(json.as_bytes());
    format!("{:x}", hasher.finalize())
}

fuzz_target!(|data: &[u8]| {
    // Deserialize from arbitrary bytes
    if let Ok(json_str) = std::str::from_utf8(data) {
        if let Ok(rule) = serde_json::from_str::<RuleIr>(json_str) {
            // Round-trip: serialize → deserialize → rehash
            let original_hash = canonical_hash(&rule);

            let serialized = serde_json::to_string(&rule).expect("serialize");
            if let Ok(deserialized) = serde_json::from_str::<RuleIr>(&serialized) {
                let rehashed = canonical_hash(&deserialized);

                // INVARIANT: hash must be identical after round-trip
                assert_eq!(
                    original_hash, rehashed,
                    "IR hash stability violated: {:?} vs {:?}",
                    original_hash, rehashed
                );
            }
        }
    }
});
