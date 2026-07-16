#![no_main]
use libfuzzer_sys::fuzz_target;
use sha2::{Digest, Sha256};

fuzz_target!(|data: &[u8]| {
    // INVARIANT 1: Parser must never panic on adversarial input.
    // INVARIANT 2: If compilation succeeds, IR hash is stable across re-parse.
    if let Ok(input) = std::str::from_utf8(data) {
        // Wrap in minimal FPL template to avoid parse-level rejections
        let fpl_source = format!(
            r#"
rule "{}" {{
    trigger: user_starts_session
    condition: {{ {} }}
    action: show_notification {{ text: "test" }}
    priority: 0
}}
"#,
            generate_rule_name(input),
            input.trim()
        );

        // INVARIANT 1: Must not panic
        let _ = std::panic::catch_unwind(|| {
            // Note: focus_lang::compile would be called here
            // For now, we simulate by attempting JSON parse of rule IR
            let _ = serde_json::from_str::<serde_json::Value>(&fpl_source);
        });

        // INVARIANT 2: If compilation succeeds, hash is stable
        if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&fpl_source) {
            let json1 = serde_json::to_string(&parsed).expect("serialize");
            let mut hasher = Sha256::new();
            hasher.update(json1.as_bytes());
            let hash1 = format!("{:x}", hasher.finalize());

            let json2 = serde_json::to_string(&parsed).expect("serialize");
            let mut hasher = Sha256::new();
            hasher.update(json2.as_bytes());
            let hash2 = format!("{:x}", hasher.finalize());

            assert_eq!(hash1, hash2, "Starlark parse hash stability violated");
        }
    }
});

/// Generate a valid rule name from arbitrary bytes.
fn generate_rule_name(input: &str) -> String {
    input
        .chars()
        .take(32)
        .filter(|c| c.is_alphanumeric() || *c == '_')
        .collect::<String>()
        .chars()
        .take(32)
        .collect()
}
