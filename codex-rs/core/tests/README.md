# Integration Tests

This directory contains integration tests for the `codex-core` crate that require external dependencies or API access.

## Test Files

### async_agent_executor_integration.rs

Tests DirectProcessExecutor with real AI provider CLIs (Claude, Gemini, OpenAI).

**Test Coverage:**
- ✅ Basic CLI execution (stdout, exit codes)
- ✅ Large prompt handling (>1KB via stdin piping)
- ✅ OAuth2 error detection (missing API keys)
- ✅ Multi-provider support (Anthropic, Google, OpenAI)
- ✅ Timeout handling and process cleanup
- ✅ Error handling (CommandNotFound)
- ✅ Concurrent execution (thread-safety)
- ✅ stderr capture verification

## Prerequisites

All integration tests are marked with `#[ignore]` to make them optional and CI-friendly. They only run when explicitly requested.

### Install AI CLIs

```bash
# Claude (Anthropic)
pip install anthropic-cli

# Gemini (Google)
pip install google-generativeai-cli

# OpenAI
pip install openai-cli
```

**Verification:**
```bash
# Check installations
claude --version
gemini --version
openai --version
```

### Set API Keys

```bash
# Anthropic Claude
export ANTHROPIC_API_KEY="sk-ant-api03-..."

# Google Gemini
export GOOGLE_API_KEY="AIza..."

# OpenAI
export OPENAI_API_KEY="sk-..."
```

**Persistent Configuration (Optional):**
Add to `~/.bashrc` or `~/.zshrc`:
```bash
export ANTHROPIC_API_KEY="your-key-here"
export GOOGLE_API_KEY="your-key-here"
export OPENAI_API_KEY="your-key-here"
```

## Running Tests

### Run All Integration Tests

```bash
# From repository root
cargo test -p codex-core --test async_agent_executor_integration -- --ignored

# From codex-rs directory
cd codex-rs
cargo test -p codex-core --test async_agent_executor_integration -- --ignored
```

### Run Specific Test

```bash
# Run only Claude small prompt test
cargo test -p codex-core --test async_agent_executor_integration test_claude_small_prompt -- --ignored

# Run only timeout test
cargo test -p codex-core --test async_agent_executor_integration test_openai_timeout -- --ignored

# Run only non-existent CLI test (no API key needed)
cargo test -p codex-core --test async_agent_executor_integration test_nonexistent_cli -- --ignored
```

### Run Without API Keys

```bash
# Tests will be skipped (not run)
cargo test -p codex-core --test async_agent_executor_integration

# Output: "test result: ok. 0 passed; 0 failed; 9 ignored; 0 measured"
```

## Test Details

### Test Categories

**Basic Execution Tests:**
- `test_claude_small_prompt` - Basic CLI execution with small prompt
- `test_gemini_small_prompt` - Multi-provider support verification
- `test_claude_stderr` - stderr capture verification

**Advanced Feature Tests:**
- `test_claude_large_prompt` - Large prompt (>1KB) via stdin piping
- `test_openai_timeout` - Timeout handling and process cleanup
- `test_concurrent_execution` - Thread-safety verification

**Error Handling Tests:**
- `test_claude_oauth2_error` - OAuth2 error detection (no API key)
- `test_nonexistent_cli` - CommandNotFound error handling

### Expected Behavior

**With API Keys:**
```bash
$ cargo test -p codex-core --test async_agent_executor_integration -- --ignored

running 9 tests
test test_claude_large_prompt ... ok
test test_claude_oauth2_error ... ok
test test_claude_small_prompt ... ok
test test_claude_stderr ... ok
test test_concurrent_execution ... ok
test test_gemini_small_prompt ... ok
test test_nonexistent_cli ... ok
test test_openai_timeout ... ok

test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

**Without API Keys:**
Tests will panic with helpful error messages:
```
ANTHROPIC_API_KEY not set - install ANTHROPIC_API_KEY and set API key:
 Install: pip install anthropic-cli
 Set key: export ANTHROPIC_API_KEY="your-api-key-here"
```

**Without --ignored Flag:**
```bash
$ cargo test -p codex-core --test async_agent_executor_integration

running 9 tests
test test_claude_large_prompt ... ignored
test test_claude_oauth2_error ... ignored
test test_claude_small_prompt ... ignored
test test_claude_stderr ... ignored
test test_concurrent_execution ... ignored
test test_gemini_small_prompt ... ignored
test test_nonexistent_cli ... ignored
test test_openai_timeout ... ignored

test result: ok. 0 passed; 0 failed; 9 ignored; 0 measured; 0 filtered out
```

## Troubleshooting

### "command not found: claude"

**Solution:** Install the Anthropic CLI:
```bash
pip install anthropic-cli

# Or with pipx (isolated environment)
pipx install anthropic-cli
```

### "ANTHROPIC_API_KEY not set"

**Solution:** Set your API key:
```bash
export ANTHROPIC_API_KEY="sk-ant-api03-..."
```

Get your API key from: https://console.anthropic.com/

### "Test timed out"

**Cause:** Network latency or API rate limits

**Solution:** Tests use generous 600s (10 minute) timeouts. If tests still timeout:
1. Check network connectivity
2. Verify API key is valid
3. Check API rate limits on provider dashboard

### "OAuth2Required error expected, got different error"

**Cause:** CLI behavior changed or different error format

**Solution:** Update test expectations or CLI version:
```bash
pip install --upgrade anthropic-cli
```

## CI/CD Integration

Integration tests are designed to be CI-friendly:

```yaml
# .github/workflows/test.yml
name: Tests

on: [push, pull_request]

jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - uses: actions-rs/toolchain@v1
        with:
          toolchain: stable

      # Unit tests (always run)
      - run: cargo test -p codex-core --lib

      # Integration tests (optional, only if API keys configured)
      - run: cargo test -p codex-core --test async_agent_executor_integration -- --ignored
        if: env.ANTHROPIC_API_KEY != ''
        env:
          ANTHROPIC_API_KEY: ${{ secrets.ANTHROPIC_API_KEY }}
          GOOGLE_API_KEY: ${{ secrets.GOOGLE_API_KEY }}
          OPENAI_API_KEY: ${{ secrets.OPENAI_API_KEY }}
```

**Key Points:**
- Without `--ignored` flag: Tests are skipped (0 passed, 9 ignored)
- With `--ignored` but no API keys: Tests panic with helpful messages
- With `--ignored` and API keys: Tests execute against real APIs

## Performance Expectations

**Test Execution Times:**
- Basic tests: 1-3 seconds per test (network latency dependent)
- Large prompt tests: 2-5 seconds (2KB prompt processing)
- Timeout tests: <100ms (forced timeout)
- Concurrent tests: 3-10 seconds (3 parallel executions)

**Total Suite Runtime:** ~30-60 seconds with API keys (network dependent)

## Cost Considerations

**API Usage:**
- Each test makes 1-3 API calls to the respective provider
- Total: ~15-20 API calls per full test run
- Estimated cost: <$0.01 per run (based on current pricing)

**Recommendation:**
- Run integration tests manually before PRs
- Configure CI to run only on main branch or with manual approval
- Use dedicated test API keys with spending limits

## Related Documentation

- **Unit Tests:** See `codex-rs/core/src/async_agent_executor.rs` (#[cfg(test)] module)
- **Implementation:** See `codex-rs/core/src/async_agent_executor.rs` (AsyncAgentExecutor trait + DirectProcessExecutor)
- **Design Document:** See `docs/SPEC-KIT-936-tmux-elimination/plan.md`
- **Task Breakdown:** See `docs/SPEC-KIT-936-tmux-elimination/tasks.md` (T2.4)

## Contributing

When adding new integration tests:

1. **Mark with `#[ignore]`** - Keep tests CI-friendly
2. **Check env vars gracefully** - Use `require_env_var()` helper
3. **Document prerequisites** - List required CLIs and API keys
4. **Keep tests independent** - No shared state between tests
5. **Add to README** - Document new test in this file

## License

Same as parent project (see repository root).
