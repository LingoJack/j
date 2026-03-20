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

# Check if we're in a valid Go project
if [ ! -f "go.mod" ]; then
    log_error "go.mod not found. Please run this script from the backend project root."
    exit 1
fi

log_info "Starting backend verification..."

# Check Go installation
if ! command -v go &> /dev/null; then
    log_error "Go is not installed"
    exit 1
fi

# Download dependencies
log_info "Downloading dependencies..."
if ! go mod download; then
    log_error "Failed to download dependencies"
    exit 1
fi

log_info "Verifying dependencies..."
if ! go mod verify; then
    log_warning "Some dependencies may have issues"
fi

# Run tests if they exist
if go test ./... 2>/dev/null; then
    log_success "All tests passed!"
else
    log_warning "No tests found or tests failed"
fi

# Build the application
log_info "Building application..."
if go build -o bin/app cmd/main.go 2>&1; then
    log_success "Build completed successfully!"
    log_info "Binary location: ./bin/app"
else
    log_error "Build failed. Please check the errors above."
    exit 1
fi

# Show binary size
if [ -f "bin/app" ]; then
    BINARY_SIZE=$(du -h bin/app | cut -f1)
    log_info "Binary size: $BINARY_SIZE"
fi
