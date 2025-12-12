## Install & build

### System requirements

| Requirement                 | Details                                                         |
| --------------------------- | --------------------------------------------------------------- |
| Operating systems           | macOS 12+, Ubuntu 20.04+/Debian 10+, or Windows 11 **via WSL2** |
| Git (optional, recommended) | 2.23+ for built-in PR helpers                                   |
| RAM                         | 4-GB minimum (8-GB recommended)                                 |

### Build from source

```bash
# From the repo root:
bash scripts/setup-hooks.sh
./build-fast.sh
./build-fast.sh run

# Tests:
cd codex-rs
cargo test -p codex-core
```  
