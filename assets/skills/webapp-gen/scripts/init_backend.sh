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
MODULE_NAME="${2:-github.com/example/$PROJECT_NAME}"

# Validate project name
if ! [[ "$PROJECT_NAME" =~ ^[a-zA-Z0-9_-]+$ ]]; then
    log_error "Invalid project name. Only alphanumeric characters, hyphens, and underscores are allowed."
    exit 1
fi

# Check if backend directory already exists
if [ -d "backend" ]; then
    log_error "Backend directory already exists"
    exit 1
fi

# Check Go installation
if ! command -v go &> /dev/null; then
    log_error "Go is not installed. Please install Go 1.21+ first."
    exit 1
fi

log_info "Creating Go backend project: $PROJECT_NAME"
log_info "Module name: $MODULE_NAME"

# Create backend directory structure
mkdir -p backend
cd backend || exit 1

# Initialize Go module
log_info "Initializing Go module..."
go mod init "$MODULE_NAME" || {
    log_error "Failed to initialize Go module"
    cd ..
    exit 1
}

# Create directory structure
log_info "Creating project structure..."
mkdir -p {cmd,internal/{service,repository,model,middleware},config,pkg,docs}

# Create main.go
cat > cmd/main.go << 'EOF'
package main

import (
	"log"

	"github.com/gin-gonic/gin"
)

func main() {
	router := gin.Default()

	// Health check endpoint
	router.GET("/health", func(c *gin.Context) {
		c.JSON(200, gin.H{"status": "ok"})
	})

	log.Println("Server starting on :8080")
	if err := router.Run(":8080"); err != nil {
		log.Fatalf("Failed to start server: %v", err)
	}
}
EOF

# Create go.mod dependencies
log_info "Installing dependencies..."
go get github.com/gin-gonic/gin@latest || {
    log_error "Failed to install Gin framework"
    cd ..
    exit 1
}

go get gorm.io/gorm@latest || {
    log_error "Failed to install GORM"
    cd ..
    exit 1
}

go get gorm.io/driver/mysql@latest || {
    log_error "Failed to install MySQL driver"
    cd ..
    exit 1
}

# Create a README
cat > README.md << 'EOF'
# Backend Service

Go + Gin + GORM backend for the web application.

## Project Structure

```
.
├── cmd/              # Application entry points
├── internal/
│   ├── service/      # Business logic
│   ├── repository/   # Data access layer
│   ├── model/        # Data models
│   └── middleware/   # Custom middleware
├── config/           # Configuration management
├── pkg/              # Public packages
└── docs/             # API documentation
```

## Setup

1. Install dependencies:
   ```bash
   go mod download
   ```

2. Configure database in `config/` directory

3. Run server:
   ```bash
   go run cmd/main.go
   ```

## API Documentation

See `docs/` directory for API specifications.
EOF

log_success "Backend project initialized successfully!"
log_info "Project location: backend/"
log_info "Next steps:"
log_info "  1. cd backend"
log_info "  2. Configure database connection in config/"
log_info "  3. go run cmd/main.go  (to start the server)"
log_info "  4. Implement service handlers based on API design"
