#!/bin/bash
echo "=== ACE Diagnostic ==="
echo ""
echo "1. Binary info:"
ls -lh /home/thetu/code/codex-rs/target/dev-fast/code
echo ""
echo "2. Config ACE section:"
grep -A 10 "^\[ace\]" ~/.code/config.toml
echo ""
echo "3. Gemini wrapper:"
ls -la /home/thetu/.local/bin/gemini-wrapper
echo ""
echo "4. Gemini settings.json:"
cat ~/.gemini/settings.json
lsattr ~/.gemini/settings.json 2>/dev/null || ls -la ~/.gemini/settings.json
echo ""
echo "5. ACE database:"
ls -lh ~/.code/ace/playbooks_normalized.sqlite3
sqlite3 ~/.code/ace/playbooks_normalized.sqlite3 "SELECT scope, COUNT(*) FROM playbook_bullet GROUP BY scope;"
echo ""
echo "6. Test gemini wrapper:"
/home/thetu/.local/bin/gemini-wrapper -y -m gemini-2.5-flash "Say OK" 2>&1 | head -3
echo ""
echo "7. Config gemini agent:"
grep -A 10 'name = "gemini"' ~/.code/config.toml | head -12
echo ""
echo "=== To test ACE in TUI ==="
echo "1. Make sure TUI is NOT running"
echo "2. Start fresh: cd /home/thetu/code/codex-rs && /home/thetu/code/codex-rs/target/dev-fast/code"
echo "3. Run: /speckit.plan SPEC-KIT-069"
echo "4. Look for: '⏳ Preparing prompt with ACE context...'"
