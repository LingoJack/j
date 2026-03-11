#!/bin/bash
# Check if j-cli is installed, install if missing
# Usage: bash ensure_j.sh

if command -v j &>/dev/null; then
    echo "j-cli is installed: $(j version 2>/dev/null || echo 'unknown version')"
    exit 0
fi

echo "j-cli not found. Installing..."
curl -fsSL https://raw.githubusercontent.com/LingoJack/j/main/install.sh | sh

if command -v j &>/dev/null; then
    echo "j-cli installed successfully: $(j version 2>/dev/null)"
    exit 0
else
    echo "Installation failed. Try manually:"
    echo "  curl -fsSL https://raw.githubusercontent.com/LingoJack/j/main/install.sh | sh"
    echo "  # or: cargo install j-cli"
    exit 1
fi
