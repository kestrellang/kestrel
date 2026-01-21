#!/bin/bash
set -e

HOOKS_DIR="$(git rev-parse --show-toplevel)/.git/hooks"
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"

echo "Installing pre-commit hook..."

cat > "$HOOKS_DIR/pre-commit" << 'EOF'
#!/bin/bash
set -e

echo "Running cargo fmt..."
if ! cargo fmt --check > /dev/null 2>&1; then
    echo "❌ Formatting issues found. Run 'cargo fmt' to fix."
    exit 1
fi

echo "Running cargo clippy..."
if ! cargo clippy --workspace --lib --bins --examples -- -D warnings > /dev/null 2>&1; then
    echo "❌ Clippy warnings found. Run 'cargo clippy' to see details."
    exit 1
fi

echo "✓ Pre-commit checks passed"
EOF

chmod +x "$HOOKS_DIR/pre-commit"

echo "✓ Pre-commit hook installed"
echo ""
echo "The hook will run 'cargo fmt --check' and 'cargo clippy' before each commit."
echo "To skip the hook temporarily, use: git commit --no-verify"
