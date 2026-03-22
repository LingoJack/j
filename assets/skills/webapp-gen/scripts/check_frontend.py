#!/usr/bin/env python3
"""Verify frontend: install deps if needed, run build, report output size."""

import sys
import subprocess
from pathlib import Path

# Colors
BLUE = "\033[0;34m"
GREEN = "\033[0;32m"
YELLOW = "\033[1;33m"
RED = "\033[0;31m"
NC = "\033[0m"


def log_info(msg):
    print(f"{BLUE}[INFO]{NC} {msg}")


def log_success(msg):
    print(f"{GREEN}[SUCCESS]{NC} {msg}")


def log_warning(msg):
    print(f"{YELLOW}[WARN]{NC} {msg}")


def log_error(msg):
    print(f"{RED}[ERROR]{NC} {msg}", file=sys.stderr)


def main():
    if not Path("package.json").exists():
        log_error("package.json not found. Please run this script from the frontend project root.")
        sys.exit(1)

    log_info("Starting frontend build verification...")

    # Check if node_modules exists
    if not Path("node_modules").is_dir():
        log_warning("node_modules not found. Installing dependencies...")
        if subprocess.run(["npm", "install"]).returncode != 0:
            log_error("Failed to install dependencies")
            sys.exit(1)

    # Run build
    log_info("Running build process...")
    if subprocess.run(["npm", "run", "build"]).returncode == 0:
        log_success("Build completed successfully!")
        dist = Path("dist")
        if dist.is_dir():
            total = sum(f.stat().st_size for f in dist.rglob("*") if f.is_file())
            if total >= 1024 * 1024:
                log_info(f"Build output size: {total / (1024 * 1024):.1f}M")
            else:
                log_info(f"Build output size: {total / 1024:.0f}K")
            log_info("Build files location: ./dist")
    else:
        log_error("Build failed. Please check the errors above.")
        sys.exit(1)


if __name__ == "__main__":
    main()
