#!/bin/bash

set -euo pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Logging functions
log_info() {
    echo -e "${BLUE}[INFO]${NC} $*"
}

log_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $*"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $*" >&2
}

# Validate input
if [ $# -lt 1 ]; then
    log_error "Usage: $0 <project_name>"
    exit 1
fi

PROJECT_NAME="$1"

# Validate project name
if ! [[ "$PROJECT_NAME" =~ ^[a-zA-Z0-9_-]+$ ]]; then
    log_error "Invalid project name. Only alphanumeric characters, hyphens, and underscores are allowed."
    exit 1
fi

# Check if frontend directory already exists
if [ -d "frontend/$PROJECT_NAME" ]; then
    log_error "Frontend project already exists at frontend/$PROJECT_NAME"
    exit 1
fi

log_info "Creating React + TypeScript frontend project: $PROJECT_NAME"

# Create frontend directory structure
mkdir -p frontend
cd frontend || exit 1

# Initialize Vite project with React + TypeScript
log_info "Initializing Vite project..."
echo "" | npx -y create-vite@latest "$PROJECT_NAME" --template react-ts 2>&1 | grep -v "npm warn" || true

if [ ! -d "$PROJECT_NAME" ]; then
    log_error "Failed to create Vite project"
    cd ..
    exit 1
fi

cd "$PROJECT_NAME" || exit 1

# Install dependencies
log_info "Installing npm dependencies..."
npm install --prefer-offline || {
    log_error "Failed to install dependencies"
    cd ../..
    exit 1
}

log_success "Frontend project initialized successfully!"
log_info "Project location: frontend/$PROJECT_NAME"
log_info "Next steps:"
log_info "  1. cd frontend/$PROJECT_NAME"
log_info "  2. npm run dev  (to start development server)"
log_info "  3. Implement the UI based on requirements"