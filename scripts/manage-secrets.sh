#!/bin/bash
# =============================================================================
# GK Mail - Secrets Management Script
# =============================================================================
# This script helps manage TLS certificates and secrets for production
# =============================================================================

set -e

SECRETS_DIR="${SECRETS_DIR:-./secrets}"
DOMAIN="${MAIL_DOMAIN:-localhost}"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# -----------------------------------------------------------------------------
# Helper functions
# -----------------------------------------------------------------------------

print_info() {
    echo -e "${GREEN}[INFO]${NC} $1"
}

print_warn() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

print_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

check_command() {
    if ! command -v $1 &> /dev/null; then
        print_error "$1 is required but not installed"
        exit 1
    fi
}

# -----------------------------------------------------------------------------
# Initialize secrets directory
# -----------------------------------------------------------------------------

init_secrets() {
    print_info "Initializing secrets directory..."

    mkdir -p "$SECRETS_DIR"
    chmod 700 "$SECRETS_DIR"

    # Create .gitignore to prevent accidental commits
    cat > "$SECRETS_DIR/.gitignore" <<EOF
# Ignore all secrets
*

# Except this gitignore and README
!.gitignore
!README.md
EOF

    # Create README
    cat > "$SECRETS_DIR/README.md" <<EOF
# Secrets Directory

This directory contains sensitive information that should NEVER be committed to version control.

## Files:
- \`mail_tls_cert.pem\` - TLS certificate for mail server
- \`mail_tls_key.pem\` - TLS private key for mail server
- \`proxy_tls_cert.pem\` - TLS certificate for proxy
- \`proxy_tls_key.pem\` - TLS private key for proxy
- \`openai_api_key.txt\` - OpenAI API key (optional)

## Generate self-signed certificates:
\`\`\`bash
./scripts/manage-secrets.sh generate-self-signed
\`\`\`

## Import Let's Encrypt certificates:
\`\`\`bash
./scripts/manage-secrets.sh import-letsencrypt
\`\`\`
EOF

    print_info "Secrets directory initialized at $SECRETS_DIR"
}

# -----------------------------------------------------------------------------
# Generate self-signed certificates (for development/testing)
# -----------------------------------------------------------------------------

generate_self_signed() {
    print_info "Generating self-signed TLS certificates..."
    check_command openssl

    mkdir -p "$SECRETS_DIR"

    # Generate mail server certificate
    print_info "Generating certificate for $DOMAIN..."
    openssl req -x509 -newkey rsa:4096 -nodes \
        -keyout "$SECRETS_DIR/mail_tls_key.pem" \
        -out "$SECRETS_DIR/mail_tls_cert.pem" \
        -days 365 \
        -subj "/CN=$DOMAIN/O=GK Mail/C=US"

    # Copy for proxy (can be different in production)
    cp "$SECRETS_DIR/mail_tls_cert.pem" "$SECRETS_DIR/proxy_tls_cert.pem"
    cp "$SECRETS_DIR/mail_tls_key.pem" "$SECRETS_DIR/proxy_tls_key.pem"

    # Set proper permissions
    chmod 600 "$SECRETS_DIR"/*.pem

    print_info "Self-signed certificates generated successfully"
    print_warn "These certificates are for development only!"
    print_warn "For production, use Let's Encrypt or a trusted CA"
}

# -----------------------------------------------------------------------------
# Import Let's Encrypt certificates
# -----------------------------------------------------------------------------

import_letsencrypt() {
    print_info "Importing Let's Encrypt certificates..."

    LETSENCRYPT_DIR="/etc/letsencrypt/live/$DOMAIN"

    if [ ! -d "$LETSENCRYPT_DIR" ]; then
        print_error "Let's Encrypt certificates not found at $LETSENCRYPT_DIR"
        print_info "Run: sudo certbot certonly --standalone -d $DOMAIN"
        exit 1
    fi

    mkdir -p "$SECRETS_DIR"

    # Copy certificates
    sudo cp "$LETSENCRYPT_DIR/fullchain.pem" "$SECRETS_DIR/mail_tls_cert.pem"
    sudo cp "$LETSENCRYPT_DIR/privkey.pem" "$SECRETS_DIR/mail_tls_key.pem"

    # Copy for proxy
    sudo cp "$LETSENCRYPT_DIR/fullchain.pem" "$SECRETS_DIR/proxy_tls_cert.pem"
    sudo cp "$LETSENCRYPT_DIR/privkey.pem" "$SECRETS_DIR/proxy_tls_key.pem"

    # Fix ownership and permissions
    sudo chown $USER:$USER "$SECRETS_DIR"/*.pem
    chmod 600 "$SECRETS_DIR"/*.pem

    print_info "Let's Encrypt certificates imported successfully"
}

# -----------------------------------------------------------------------------
# Verify certificates
# -----------------------------------------------------------------------------

verify_certs() {
    print_info "Verifying certificates..."
    check_command openssl

    for cert in mail_tls_cert.pem proxy_tls_cert.pem; do
        if [ -f "$SECRETS_DIR/$cert" ]; then
            print_info "Checking $cert..."
            openssl x509 -in "$SECRETS_DIR/$cert" -text -noout | grep -E "Subject:|Issuer:|Not Before:|Not After:"
            echo ""
        else
            print_warn "$cert not found"
        fi
    done
}

# -----------------------------------------------------------------------------
# Set API keys
# -----------------------------------------------------------------------------

set_openai_key() {
    print_info "Setting OpenAI API key..."

    read -p "Enter your OpenAI API key: " -s api_key
    echo ""

    if [ -z "$api_key" ]; then
        print_error "API key cannot be empty"
        exit 1
    fi

    echo -n "$api_key" > "$SECRETS_DIR/openai_api_key.txt"
    chmod 600 "$SECRETS_DIR/openai_api_key.txt"

    print_info "OpenAI API key saved to $SECRETS_DIR/openai_api_key.txt"
}

# -----------------------------------------------------------------------------
# Clean secrets
# -----------------------------------------------------------------------------

clean_secrets() {
    print_warn "This will delete all secrets in $SECRETS_DIR"
    read -p "Are you sure? (yes/no): " confirm

    if [ "$confirm" != "yes" ]; then
        print_info "Cancelled"
        exit 0
    fi

    rm -f "$SECRETS_DIR"/*.pem "$SECRETS_DIR"/*.txt
    print_info "Secrets cleaned"
}

# -----------------------------------------------------------------------------
# Main menu
# -----------------------------------------------------------------------------

show_help() {
    cat <<EOF
GK Mail - Secrets Management

Usage: $0 <command>

Commands:
    init                 Initialize secrets directory
    generate-self-signed Generate self-signed TLS certificates (dev/test)
    import-letsencrypt   Import Let's Encrypt certificates (production)
    verify               Verify existing certificates
    set-openai-key       Set OpenAI API key
    clean                Remove all secrets
    help                 Show this help message

Examples:
    # For development
    $0 init
    $0 generate-self-signed

    # For production
    $0 init
    $0 import-letsencrypt
    $0 set-openai-key

Environment variables:
    SECRETS_DIR          Path to secrets directory (default: ./secrets)
    MAIL_DOMAIN          Domain name for certificates (default: localhost)
EOF
}

# -----------------------------------------------------------------------------
# Main
# -----------------------------------------------------------------------------

case "${1:-}" in
    init)
        init_secrets
        ;;
    generate-self-signed)
        generate_self_signed
        ;;
    import-letsencrypt)
        import_letsencrypt
        ;;
    verify)
        verify_certs
        ;;
    set-openai-key)
        set_openai_key
        ;;
    clean)
        clean_secrets
        ;;
    help|--help|-h)
        show_help
        ;;
    *)
        print_error "Unknown command: ${1:-}"
        echo ""
        show_help
        exit 1
        ;;
esac
