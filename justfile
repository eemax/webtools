check:
    cargo fmt --check
    cargo clippy --all-targets -- -D warnings
    cargo test

test-live:
    cargo test --test live -- --ignored
