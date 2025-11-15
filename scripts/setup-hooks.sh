#!/bin/bash
# Setup git hooks for policy compliance (SPEC-941)
#
# Configures git to use .githooks/ directory for custom hooks.
# Run this script once after cloning the repository.

set -e

echo "🔧 Setting up git hooks..."

# Configure git to use .githooks/
git config core.hooksPath .githooks

# Verify hooks are executable
if [ ! -x .githooks/pre-commit ]; then
    echo "Making pre-commit hook executable..."
    chmod +x .githooks/pre-commit
fi

echo "✅ Git hooks configured successfully"
echo ""
echo "Installed hooks:"
echo "  - pre-commit: Policy compliance checks (SPEC-KIT-072)"
echo ""
echo "To bypass hooks (emergencies only): git commit --no-verify"
