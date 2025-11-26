# =============================================================================
# GK Mail - Makefile
# =============================================================================

.PHONY: help build up down logs dev prod clean test

# Default target
help:
	@echo "GK Mail - Available commands:"
	@echo ""
	@echo "  make dev        - Start development environment"
	@echo "  make prod       - Start production environment"
	@echo "  make build      - Build all Docker images"
	@echo "  make up         - Start all services (production)"
	@echo "  make down       - Stop all services"
	@echo "  make logs       - Follow logs from all services"
	@echo "  make clean      - Remove all containers, volumes, and images"
	@echo "  make test       - Run tests"
	@echo "  make rust-build - Build Rust binaries locally"
	@echo ""

# =============================================================================
# Development
# =============================================================================

dev:
	@echo "Starting development environment..."
	@mkdir -p data/maildir data/queue data/db
	docker compose -f docker compose.dev.yml up -d --build
	@echo ""
	@echo "Services started:"
	@echo "  - Web UI:      http://localhost:5173"
	@echo "  - AI Runtime:  ws://localhost:8888/ws"
	@echo "  - MCP Server:  http://localhost:8090"
	@echo "  - SMTP:        localhost:2525"
	@echo "  - IMAP:        localhost:1993"
	@echo ""
	@echo "Make sure Ollama is running on your host!"

dev-logs:
	docker compose -f docker compose.dev.yml logs -f

dev-down:
	docker compose -f docker compose.dev.yml down

# =============================================================================
# Production
# =============================================================================

prod: build up
	@echo "Production environment started!"

build:
	@echo "Building Docker images..."
	docker compose build

up:
	@echo "Starting production services..."
	docker compose up -d
	@echo ""
	@echo "Services started:"
	@echo "  - Web UI:  http://localhost:80"
	@echo ""

down:
	docker compose down

logs:
	docker compose logs -f

# =============================================================================
# Maintenance
# =============================================================================

clean:
	@echo "Cleaning up..."
	docker compose -f docker compose.yml down -v --rmi local 2>/dev/null || true
	docker compose -f docker compose.dev.yml down -v --rmi local 2>/dev/null || true
	rm -rf data/
	@echo "Cleanup complete!"

# =============================================================================
# Local Development (without Docker)
# =============================================================================

rust-build:
	cargo build --release --workspace

rust-test:
	cargo test --workspace

local-run:
	@echo "Starting services locally..."
	@echo "1. Start mail-rs:        ./target/release/mail-rs"
	@echo "2. Start mcp-mail-server: ./target/release/mcp-mail-server"
	@echo "3. Start ai-runtime:      OLLAMA_MODEL=llama3.1:8b ./target/release/ai-runtime"
	@echo "4. Start web-ui:          cd web-ui && npm run dev"

# =============================================================================
# Utilities
# =============================================================================

add-user:
	@read -p "Email: " email; \
	read -s -p "Password: " pass; \
	echo ""; \
	docker compose exec mail-rs /app/mail-rs add-user $$email $$pass --db /data/db/users.db

shell-mail:
	docker compose exec mail-rs /bin/sh

shell-ai:
	docker compose exec ai-runtime /bin/sh
