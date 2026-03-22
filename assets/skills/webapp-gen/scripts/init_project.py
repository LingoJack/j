#!/usr/bin/env python3
"""Initialize a new webapp project with Makefile, .gitignore and git repo."""

import subprocess
import shutil
from pathlib import Path

# Colors
BLUE = "\033[0;34m"
GREEN = "\033[0;32m"
RED = "\033[0;31m"
NC = "\033[0m"

def main():
    project_dir = Path.cwd()
    project_name = project_dir.name

    # Check if already initialized
    if (project_dir / ".git").exists():
        log_error("Git repository already exists in current directory.")
        sys.exit(1)

    # Locate skill assets directory (relative to this script)
    script_dir = Path(__file__).resolve().parent
    assets_dir = script_dir / ".." / "assets"

    log_info(f"Initializing project in current directory: {project_name}")

    # Copy template files
    shutil.copy2(assets_dir / "Makefile.template", project_dir / "Makefile")
    log_info("Created Makefile")

    shutil.copy2(assets_dir / "gitignore.template", project_dir / ".gitignore")
    log_info("Created .gitignore")

    # Create docs directory
    (project_dir / "docs").mkdir(parents=True, exist_ok=True)
    log_info("Created docs/")

    # Initialize git repository
    subprocess.run(["git", "init", "-q"], cwd=project_dir, check=True)
    subprocess.run(["git", "add", "."], cwd=project_dir, check=True)
    subprocess.run(["git", "commit", "-q", "-m", f"init: project scaffold for {project_name}"], cwd=project_dir, check=True)

    log_success("Project initialized successfully!")


def log_info(msg):
    print(f"{BLUE}[INFO]{NC} {msg}")


def log_success(msg):
    print(f"{GREEN}[SUCCESS]{NC} {msg}")


def log_error(msg):
    print(f"{RED}[ERROR]{NC} {msg}", file=sys.stderr)


if __name__ == "__main__":
    import sys
    main()
