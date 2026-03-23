#!/usr/bin/env python3
"""Initialize a new webapp project with Makefile, .gitignore and git repo."""

import subprocess
import shutil
from pathlib import Path

# Colors
BLUE = "\033[0;34m"
GREEN = "\033[0;32m"
RED = "\033[0;31m"
YELLOW = "\033[0;33m"
NC = "\033[0m"

class Logger:
    def info(self, msg):
        print(f"{BLUE}[INFO]{NC} {msg}")

    def success(self, msg):
        print(f"{GREEN}[SUCCESS]{NC} {msg}")
        
    def error(self, msg):
        print(f"{RED}[ERROR]{NC} {msg}", file=sys.stderr)
        
    def warning(self, msg):
        print(f"{YELLOW}[WARNING]{NC} {msg}")

log = Logger()

def main():
    project_dir = Path.cwd()
    project_name = project_dir.name

    # Check if already initialized
    if (project_dir / ".git").exists():
        log.error("Git repository already exists in current directory.")
        sys.exit(1)

    # Locate skill assets directory (relative to this script)
    script_dir = Path(__file__).resolve().parent
    assets_dir = script_dir / ".." / "assets"

    log.info(f"Initializing project in current directory: {project_name}")

    # Copy template files
    shutil.copy2(assets_dir / "Makefile", project_dir / "Makefile")
    log.info("Created Makefile")

    shutil.copy2(assets_dir / "gitignore", project_dir / ".gitignore")
    log.info("Created .gitignore")

    # Create docs directory
    (project_dir / "docs").mkdir(parents=True, exist_ok=True)
    log.info("Created docs/")

    # Initialize git repository
    subprocess.run(["git", "init", "-q"], cwd=project_dir, check=True)
    subprocess.run(["git", "branch", "-M", "main"], cwd=project_dir, check=True)
    subprocess.run(["git", "add", "."], cwd=project_dir, check=True)
    subprocess.run(["git", "commit", "-q", "-m", f"init: project scaffold for {project_name}"], cwd=project_dir, check=True)

    log.success("Project initialized successfully!")


if __name__ == "__main__":
    import sys
    main()
