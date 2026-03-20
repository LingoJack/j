#!/bin/bash

set -euo pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Logging functions
log_info() {
    echo -e "${BLUE}[INFO]${NC} $*"
}

log_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $*"
}

log_warning() {
    echo -e "${YELLOW}[WARN]${NC} $*"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $*" >&2
}

# Check if we're in a valid project directory
if [ ! -f "package.json" ]; then
    log_error "package.json not found. Please run this script from the frontend project root."
    exit 1
fi

log_info "Starting frontend build verification..."

# Check if node_modules exists
if [ ! -d "node_modules" ]; then
    log_warning "node_modules not found. Installing dependencies..."
    npm install || {
        log_error "Failed to install dependencies"
        exit 1
    }
fi

# Run build
log_info "Running build process..."
if npm run build 2>&1; then
    log_success "Build completed successfully!"

    # Show build output size
    if [ -d "dist" ]; then
        DIST_SIZE=$(du -sh dist | cut -f1)
        log_info "Build output size: $DIST_SIZE"
        log_info "Build files location: ./dist"
    fi
else
    log_error "Build failed. Please check the errors above."
    exit 1
fi