# GK Mail Server - Development Commands
# Usage: just <command>
# Install just: cargo install just

# Default recipe (show help)
default:
    @just --list

# ========== DEVELOPMENT ==========

# Start all services in development mode
dev:
    @echo "Starting all services in development mode..."
    @just dev-mail &
    @sleep 2
    @just dev-mcp &
    @sleep 2
    @just dev-ai &
    @echo "All services started!"
    @echo "Mail Server: http://localhost:8080"
    @echo "Admin Panel: http://localhost:8080/admin/login"
    @echo "Chat: http://localhost:8080/chat/login"
    @wait

# Start mail server in development mode
dev-mail:
    @echo "Starting mail server..."
    RUST_LOG=info cargo run --bin mail-rs -- --config mail-rs/config.toml

# Start MCP server in development mode
dev-mcp:
    @echo "Starting MCP server..."
    cd mcp-mail-server && RUST_LOG=info cargo run

# Start AI runtime in development mode
dev-ai:
    @echo "Starting AI runtime..."
    cd ai-runtime && RUST_LOG=info cargo run

# Start only the mail server (for admin interface testing)
dev-mail-only:
    @echo "Starting mail server only..."
    RUST_LOG=debug cargo run --bin mail-rs -- --config mail-rs/config.toml

# ========== BUILD ==========

# Build all packages in debug mode
build:
    @echo "Building all packages..."
    cargo build

# Build all packages in release mode
build-release:
    @echo "Building all packages in release mode..."
    cargo build --release

# Build mail-rs only
build-mail:
    cargo build --bin mail-rs

# Build with verbose output
build-verbose:
    cargo build --verbose

# ========== TESTING ==========

# Run all tests
test:
    @echo "Running all tests..."
    cargo test --all

# Run tests with output
test-verbose:
    cargo test --all -- --nocapture

# Run mail-rs tests only
test-mail:
    cargo test --package mail-rs

# Run SMTP integration tests
test-smtp:
    cargo test --package mail-rs --test smtp_test

# Run MCP integration tests
test-mcp:
    cd mcp-mail-server && cargo test --test integration_test

# Run AI runtime tests
test-ai:
    cd ai-runtime && cargo test --test integration_test

# Run end-to-end tests
test-e2e:
    @echo "Running end-to-end tests..."
    ./test_e2e.sh

# ========== USER MANAGEMENT ==========

# Create a new user (usage: just create-user admin@example.com password123)
create-user email password:
    @echo "Creating user: {{email}}"
    cargo run --bin mail-user -- add {{email}} {{password}}

# List all users
list-users:
    @echo "Listing all users..."
    cargo run --bin mail-user -- list

# Delete a user (usage: just delete-user admin@example.com)
delete-user email:
    @echo "Deleting user: {{email}}"
    cargo run --bin mail-user -- delete {{email}}

# Create default admin user for development
create-admin:
    @echo "Creating default admin user..."
    @just create-user admin@delfour.co password123
    @echo "Admin user created: admin@delfour.co / password123"

# ========== DATABASE ==========

# Reset all databases (WARNING: deletes all data)
reset-db:
    @echo "Resetting all databases..."
    rm -f mail-rs/data/users.db
    rm -f mail-rs/data/queue.db
    rm -f ai-runtime/summaries.db
    @echo "Databases reset. Run 'just create-admin' to create admin user."

# Backup databases
backup-db:
    @echo "Backing up databases..."
    mkdir -p backups
    cp mail-rs/data/users.db backups/users_$(date +%Y%m%d_%H%M%S).db || true
    cp mail-rs/data/queue.db backups/queue_$(date +%Y%m%d_%H%M%S).db || true
    cp ai-runtime/summaries.db backups/summaries_$(date +%Y%m%d_%H%M%S).db || true
    @echo "Databases backed up to backups/"

# ========== CLEAN ==========

# Clean build artifacts
clean:
    @echo "Cleaning build artifacts..."
    cargo clean

# Clean everything (build + data + logs)
clean-all:
    @echo "Cleaning everything..."
    cargo clean
    rm -rf mail-rs/data/maildir/*
    rm -f mail-rs/data/*.db
    rm -f ai-runtime/summaries.db
    rm -f *.log

# Clean only maildir
clean-maildir:
    @echo "Cleaning maildir..."
    rm -rf mail-rs/data/maildir/*

# ========== CODE QUALITY ==========

# Format all code
fmt:
    @echo "Formatting code..."
    cargo fmt --all

# Check code formatting
fmt-check:
    cargo fmt --all -- --check

# Run clippy linter
lint:
    @echo "Running clippy..."
    cargo clippy --all-targets --all-features

# Run clippy with automatic fixes
lint-fix:
    cargo clippy --all-targets --all-features --fix --allow-dirty

# Check code without building
check:
    cargo check --all

# ========== DEPENDENCIES ==========

# Update dependencies
update:
    @echo "Updating dependencies..."
    cargo update

# Show dependency tree
deps:
    cargo tree

# Audit dependencies for security issues
audit:
    cargo audit

# ========== SETUP ==========

# Initial setup (install dependencies, create admin user)
setup:
    @echo "Setting up GK Mail Server..."
    @echo "1. Checking Rust installation..."
    @rustc --version || (echo "Please install Rust: https://rustup.rs" && exit 1)
    @echo "2. Checking Ollama installation..."
    @which ollama || (echo "Please install Ollama: https://ollama.com" && exit 1)
    @echo "3. Pulling LLM model..."
    ollama pull llama3.1:8b
    @echo "4. Building project..."
    @just build
    @echo "5. Creating data directories..."
    mkdir -p mail-rs/data/maildir
    mkdir -p backups
    @echo "6. Creating admin user..."
    @just create-admin
    @echo ""
    @echo "Setup complete! Run 'just dev' to start all services."

# Check if all prerequisites are installed
check-deps:
    @echo "Checking prerequisites..."
    @rustc --version || (echo "❌ Rust not found" && exit 1)
    @cargo --version || (echo "❌ Cargo not found" && exit 1)
    @which ollama > /dev/null || (echo "❌ Ollama not found" && exit 1)
    @which sqlite3 > /dev/null || (echo "⚠️  SQLite3 not found (optional)" && exit 0)
    @echo "✅ All prerequisites installed!"

# ========== DOCKER ==========

# Build Docker image (future)
docker-build:
    @echo "Docker support coming soon..."

# Run with Docker Compose (future)
docker-up:
    @echo "Docker support coming soon..."

# ========== LOGS ==========

# Show mail server logs
logs-mail:
    tail -f mail-rs/logs/*.log || echo "No logs found"

# Show all logs
logs:
    tail -f **/*.log

# ========== ADMIN INTERFACE ==========

# Open admin interface in browser
admin:
    @echo "Opening admin interface..."
    @echo "URL: http://localhost:8080/admin/login"
    @echo "Default credentials: admin@delfour.co / password123"
    xdg-open http://localhost:8080/admin/login 2>/dev/null || open http://localhost:8080/admin/login 2>/dev/null || echo "Please open http://localhost:8080/admin/login in your browser"

# Open chat interface in browser
chat:
    @echo "Opening chat interface..."
    @echo "URL: http://localhost:8080/chat/login"
    xdg-open http://localhost:8080/chat/login 2>/dev/null || open http://localhost:8080/chat/login 2>/dev/null || echo "Please open http://localhost:8080/chat/login in your browser"

# ========== UTILITIES ==========

# Show project statistics
stats:
    @echo "Project Statistics:"
    @echo "=================="
    @echo "Lines of code:"
    @find . -name "*.rs" -not -path "./target/*" | xargs wc -l | tail -1
    @echo ""
    @echo "Number of Rust files:"
    @find . -name "*.rs" -not -path "./target/*" | wc -l
    @echo ""
    @echo "Tests:"
    @cargo test --all -- --list | grep "test" | wc -l

# Generate API documentation
docs:
    cargo doc --open --no-deps

# Watch and rebuild on file changes (requires cargo-watch)
watch:
    cargo watch -x "build --bin mail-rs"

# Install development tools
install-tools:
    @echo "Installing development tools..."
    cargo install cargo-watch
    cargo install cargo-audit
    cargo install cargo-tree
    @echo "Tools installed!"
