#!/usr/bin/env python3
"""Initialize a React + TypeScript + Tailwind CSS v4 frontend project."""

import sys
import re
import subprocess
from pathlib import Path

# Colors
BLUE = "\033[0;34m"
GREEN = "\033[0;32m"
RED = "\033[0;31m"
NC = "\033[0m"


def log_info(msg):
    print(f"{BLUE}[INFO]{NC} {msg}")


def log_success(msg):
    print(f"{GREEN}[SUCCESS]{NC} {msg}")


def log_error(msg):
    print(f"{RED}[ERROR]{NC} {msg}", file=sys.stderr)


def run(cmd, **kwargs):
    """Run a command, exit on failure."""
    result = subprocess.run(cmd, **kwargs)
    if result.returncode != 0:
        log_error(f"Command failed: {' '.join(cmd)}")
        sys.exit(1)
    return result


def main():
    if len(sys.argv) < 2:
        log_error(f"Usage: {sys.argv[0]} <project_name>")
        sys.exit(1)

    project_name = sys.argv[1]

    if not re.match(r"^[a-zA-Z0-9_-]+$", project_name):
        log_error("Invalid project name. Only alphanumeric characters, hyphens, and underscores are allowed.")
        sys.exit(1)

    frontend_dir = Path("frontend") / project_name
    if frontend_dir.exists():
        log_error(f"Frontend project already exists at frontend/{project_name}")
        sys.exit(1)

    log_info(f"Creating React + TypeScript frontend project: {project_name}")

    Path("frontend").mkdir(exist_ok=True)

    # Initialize Vite project with React + TypeScript
    log_info("Initializing Vite project...")
    subprocess.run(
        ["npx", "-y", "create-vite@latest", project_name, "--template", "react-ts"],
        cwd="frontend",
        input=b"\n",
    )

    if not frontend_dir.is_dir():
        log_error("Failed to create Vite project")
        sys.exit(1)

    # Install dependencies
    log_info("Installing npm dependencies...")
    run(["npm", "install", "--prefer-offline"], cwd=frontend_dir)

    # Install Tailwind CSS v4
    log_info("Installing Tailwind CSS v4...")
    run(["npm", "install", "-D", "tailwindcss", "@tailwindcss/vite"], cwd=frontend_dir)

    # Update vite.config.ts
    log_info("Configuring Vite with Tailwind CSS plugin...")
    (frontend_dir / "vite.config.ts").write_text("""\
import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react'
import tailwindcss from '@tailwindcss/vite'

// https://vite.dev/config/
export default defineConfig({
  plugins: [
    react(),
    tailwindcss(),
  ],
})
""")

    # Update index.css
    log_info("Setting up Tailwind CSS styles...")
    (frontend_dir / "src" / "index.css").write_text('@import "tailwindcss";\n')

    # Clean up default App.css
    app_css = frontend_dir / "src" / "App.css"
    if app_css.exists():
        app_css.unlink()

    # Update App.tsx
    (frontend_dir / "src" / "App.tsx").write_text("""\
function App() {
  return (
    <div className="min-h-screen flex items-center justify-center bg-gray-100">
      <h1 className="text-4xl font-bold text-blue-600">Hello, World!</h1>
    </div>
  )
}

export default App
""")

    log_success("Frontend project initialized successfully!")
    log_info(f"Project location: frontend/{project_name}")
    log_info("Next steps:")
    log_info(f"  1. cd frontend/{project_name}")
    log_info("  2. run `npm run dev` with `BackgroudRun` tool (to start development server)")
    log_info("  3. Implement the UI based on requirements")


if __name__ == "__main__":
    main()
