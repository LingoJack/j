#!/usr/bin/env python3
"""Initialize a new webapp project with Makefile, .gitignore and git repo."""

import sys
import re
import subprocess
import shutil
from pathlib import Path

# Colors
BLUE = "\033[0;34m"
GREEN = "\033[0;32m"
RED = "\033[0;31m"
NC = "\033[0m"

def main():
    if len(sys.argv) < 2:
        log_error(f"Usage: {sys.argv[0]} <project_name>")
        sys.exit(1)

    project_name = sys.argv[1]

    # Validate project name
    if not re.match(r"^[a-zA-Z0-9_-]+$", project_name):
        log_error("Invalid project name. Only alphanumeric characters, hyphens, and underscores are allowed.")
        sys.exit(1)

    # Check if project directory already exists
    project_dir = Path(project_name)
    if project_dir.exists():
        log_error(f"Project directory already exists: {project_name}")
        sys.exit(1)

    # Locate skill assets directory (relative to this script)
    script_dir = Path(__file__).resolve().parent
    assets_dir = script_dir / ".." / "assets"

    log_info(f"Creating project: {project_name}")

    # Create project directory
    project_dir.mkdir(parents=True)

    # Copy template files
    shutil.copy2(assets_dir / "Makefile.template", project_dir / "Makefile")
    log_info("Created Makefile")
    
    shutil.copy2(assets_dir / ".gitignore.template", project_dir / ".gitignore")
    log_info("Created .gitignore")

    # Create docs directory
    (project_dir / "docs").mkdir(parents=True)
    log_info("Created docs/")

    # Initialize git repository
    subprocess.run(["git", "init", "-q"], cwd=project_dir, check=True)
    subprocess.run(["git", "add", "."], cwd=project_dir, check=True)
    subprocess.run(["git", "commit", "-q", "-m", f"init: project scaffold for {project_name}"], cwd=project_dir, check=True)

    log_success("Project initialized successfully!")
    log_info(f"Project location: {project_name}/")




def log_info(msg):
    print(f"{BLUE}[INFO]{NC} {msg}")


def log_success(msg):
    print(f"{GREEN}[SUCCESS]{NC} {msg}")


def log_error(msg):
    print(f"{RED}[ERROR]{NC} {msg}", file=sys.stderr)
    
    
    
if __name__ == "__main__":
    main()
