# Adding Slash Commands

Guide to adding new `/command` to the spec-kit framework.

---

## Command Registry Pattern

**Location**: `codex-rs/tui/src/chatwidget/spec_kit/command_registry.rs`

**Pattern**: Command registry maps `/command` → handler function

---

## Step-by-Step Guide

### 1. Define Command Enum

**File**: `command_registry.rs`

```rust
#[derive(Debug, Clone, PartialEq)]
pub enum SpecKitCommand {
    // Existing commands
    New,
    Plan,
    Status,
    // Add your command
    MyNewCommand { arg1: String },
}
```

---

### 2. Add to Registry

```rust
pub fn parse_speckit_command(input: &str) -> Option<SpecKitCommand> {
    if input.starts_with("/speckit.mynew ") {
        let args = input.strip_prefix("/speckit.mynew ")?.trim();
        return Some(SpecKitCommand::MyNewCommand {
            arg1: args.to_string()
        });
    }
    // ... existing commands
    None
}
```

---

### 3. Create Handler

**File**: `command_handlers.rs`

```rust
pub fn handle_my_new_command(
    spec_id: &str,
    config: &SpecKitConfig,
) -> Result<String> {
    // Implementation
    let result = do_something(spec_id)?;

    // Return formatted response
    Ok(format!("Command executed: {}", result))
}
```

---

### 4. Wire to Routing

**File**: `routing.rs` (or main handler)

```rust
match command {
    SpecKitCommand::MyNewCommand { arg1 } => {
        handle_my_new_command(&arg1, config)?
    }
    // ... existing commands
}
```

---

### 5. Add Tests

**File**: `command_registry_tests.rs`

```rust
#[test]
fn test_parse_mynew_command() {
    let input = "/speckit.mynew test-arg";
    let cmd = parse_speckit_command(input);

    assert_eq!(
        cmd,
        Some(SpecKitCommand::MyNewCommand {
            arg1: "test-arg".to_string()
        })
    );
}

#[test]
fn test_handle_mynew_command() {
    let result = handle_my_new_command("SPEC-TEST", &default_config());
    assert!(result.is_ok());
}
```

---

### 6. Add Documentation

**Update**: `docs/SPEC-DOC-003/content/command-reference.md`

```markdown
### /speckit.mynew

**Purpose**: Brief description

**Usage**:
\`\`\`bash
/speckit.mynew <arg>
\`\`\`

**Example**:
\`\`\`bash
/speckit.mynew test-value
\`\`\`

**Output**: Description of output
```

---

## Example: Complete Command

**Command**: `/speckit.hello <name>`

### 1. Enum

```rust
pub enum SpecKitCommand {
    Hello { name: String },
}
```

### 2. Parser

```rust
if input.starts_with("/speckit.hello ") {
    let name = input.strip_prefix("/speckit.hello ")?.trim().to_string();
    return Some(SpecKitCommand::Hello { name });
}
```

### 3. Handler

```rust
pub fn handle_hello(name: &str) -> Result<String> {
    Ok(format!("Hello, {}!", name))
}
```

### 4. Routing

```rust
SpecKitCommand::Hello { name } => {
    handle_hello(&name)?
}
```

### 5. Test

```rust
#[test]
fn test_hello_command() {
    let result = handle_hello("World");
    assert_eq!(result.unwrap(), "Hello, World!");
}
```

---

## Summary

**Steps**:
1. Add to command enum
2. Parse in registry
3. Create handler
4. Wire to routing
5. Add tests
6. Update docs

**Files Modified**:
- `command_registry.rs`
- `command_handlers.rs`
- `routing.rs` (or handler.rs)
- `*_tests.rs`

**Next**: [Debugging Guide](debugging-guide.md)
