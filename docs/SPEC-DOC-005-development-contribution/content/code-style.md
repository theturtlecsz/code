# Code Style Guide

Rust code style, formatting, and linting guidelines.

---

## rustfmt (Formatting)

### Configuration

**File**: `codex-rs/rustfmt.toml`

**Key Settings**:
- Edition: 2024
- Max width: 100
- Tab spaces: 4

### Format Code

```bash
cd codex-rs
cargo fmt --all
```

### Check Formatting

```bash
cargo fmt --all -- --check
```

**Pre-commit hook**: Automatically runs format check

---

## Clippy (Linting)

### Run Clippy

```bash
cd codex-rs
cargo clippy --workspace --all-targets --all-features -- -D warnings
```

**Flags**:
- `--all-targets`: Check tests, benches, examples
- `--all-features`: Check all feature combinations
- `-D warnings`: Treat warnings as errors

### Common Clippy Fixes

**Unused imports**:
```rust
// Bad
use std::collections::HashMap;

// Good (if unused, remove)
```

**Unnecessary clones**:
```rust
// Bad
let s = string.clone();

// Good (if ownership not needed)
let s = &string;
```

---

## Code Guidelines

### Naming Conventions

**Functions**: snake_case
```rust
fn calculate_total() { }
```

**Types**: PascalCase
```rust
struct UserAccount { }
enum RequestStatus { }
```

**Constants**: SCREAMING_SNAKE_CASE
```rust
const MAX_RETRIES: usize = 3;
```

---

### Documentation

**Public APIs**:
```rust
/// Calculates the total cost with tax
///
/// # Arguments
/// * `subtotal` - Base amount before tax
/// * `tax_rate` - Tax rate (0.0-1.0)
///
/// # Returns
/// Total amount including tax
pub fn calculate_total(subtotal: f64, tax_rate: f64) -> f64 {
    subtotal * (1.0 + tax_rate)
}
```

---

### Error Handling

**Use Result**:
```rust
// Good
fn parse_config(path: &Path) -> Result<Config> {
    let contents = fs::read_to_string(path)?;
    let config: Config = toml::from_str(&contents)?;
    Ok(config)
}

// Bad
fn parse_config(path: &Path) -> Config {
    let contents = fs::read_to_string(path).unwrap(); // ❌
    toml::from_str(&contents).unwrap() // ❌
}
```

---

## Allowed Lints

**workspace** (Cargo.toml):
```toml
[workspace.lints.clippy]
unwrap_used = "warn"
expect_used = "warn"
panic = "warn"
```

**Override in tests**:
```rust
#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    // Tests can use .unwrap()
}
```

---

## Summary

**Format**: `cargo fmt --all`
**Lint**: `cargo clippy --workspace --all-targets --all-features -- -D warnings`
**Conventions**: snake_case functions, PascalCase types, document public APIs

**Next**: [Pre-Commit Hooks](pre-commit-hooks.md)
