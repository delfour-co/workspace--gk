#!/bin/bash
#
# Mail-rs Automatic Installation Script
# Mail-in-a-Box equivalent for mail-rs
#
# Usage: sudo ./install.sh
#

set -e  # Exit on error

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Configuration variables
DOMAIN=""
EMAIL=""
HOSTNAME=""
IP_ADDRESS=""
INSTALL_DIR="/opt/mail-rs"
DATA_DIR="/var/mail"
BACKUP_DIR="/var/backups/mail-rs"
LOG_DIR="/var/log/mail-rs"
SYSTEMD_DIR="/etc/systemd/system"

# Print colored message
print_info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

print_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

print_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

print_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Print header
print_header() {
    echo ""
    echo "================================================"
    echo "  Mail-rs Installation Script"
    echo "  Mail-in-a-Box Equivalent"
    echo "================================================"
    echo ""
}

# Check if running as root
check_root() {
    if [ "$EUID" -ne 0 ]; then
        print_error "This script must be run as root"
        exit 1
    fi
    print_success "Running as root"
}

# Check system requirements
check_system_requirements() {
    print_info "Checking system requirements..."

    # Check OS
    if [ ! -f /etc/os-release ]; then
        print_error "Cannot determine OS"
        exit 1
    fi

    . /etc/os-release
    print_success "OS: $NAME $VERSION"

    # Check minimum RAM (2GB recommended)
    total_mem=$(free -m | awk '/^Mem:/{print $2}')
    if [ "$total_mem" -lt 1024 ]; then
        print_warning "System has less than 2GB RAM ($total_mem MB)"
        print_warning "Mail server may have performance issues"
    else
        print_success "Memory: ${total_mem}MB"
    fi

    # Check disk space (minimum 10GB free)
    free_space=$(df -BG / | awk 'NR==2{print $4}' | sed 's/G//')
    if [ "$free_space" -lt 10 ]; then
        print_warning "Less than 10GB free disk space (${free_space}GB available)"
    else
        print_success "Disk space: ${free_space}GB available"
    fi
}

# Install dependencies
install_dependencies() {
    print_info "Installing system dependencies..."

    # Detect package manager
    if command -v apt-get &> /dev/null; then
        # Debian/Ubuntu
        apt-get update
        apt-get install -y \
            curl \
            git \
            build-essential \
            pkg-config \
            libssl-dev \
            certbot \
            openssl \
            ca-certificates

        print_success "Dependencies installed (apt)"

    elif command -v dnf &> /dev/null; then
        # Fedora/RHEL
        dnf install -y \
            curl \
            git \
            gcc \
            gcc-c++ \
            make \
            pkgconfig \
            openssl-devel \
            certbot \
            openssl \
            ca-certificates

        print_success "Dependencies installed (dnf)"

    elif command -v yum &> /dev/null; then
        # CentOS/older RHEL
        yum install -y \
            curl \
            git \
            gcc \
            gcc-c++ \
            make \
            pkgconfig \
            openssl-devel \
            certbot \
            openssl \
            ca-certificates

        print_success "Dependencies installed (yum)"

    else
        print_error "Unsupported package manager"
        exit 1
    fi
}

# Install Rust
install_rust() {
    print_info "Checking Rust installation..."

    if command -v cargo &> /dev/null; then
        print_success "Rust already installed: $(rustc --version)"
        return
    fi

    print_info "Installing Rust..."
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y

    # Source cargo env
    . "$HOME/.cargo/env"

    print_success "Rust installed: $(rustc --version)"
}

# Gather configuration
gather_configuration() {
    print_info "Gathering configuration..."

    # Get domain
    read -p "Enter your domain name (e.g., example.com): " DOMAIN
    if [ -z "$DOMAIN" ]; then
        print_error "Domain name is required"
        exit 1
    fi

    # Get email for Let's Encrypt
    read -p "Enter admin email address: " EMAIL
    if [ -z "$EMAIL" ]; then
        print_error "Email address is required"
        exit 1
    fi

    # Auto-detect hostname
    HOSTNAME=$(hostname -f 2>/dev/null || hostname)
    read -p "Mail server hostname [$HOSTNAME]: " input_hostname
    if [ -n "$input_hostname" ]; then
        HOSTNAME="$input_hostname"
    fi

    # Auto-detect IP
    IP_ADDRESS=$(curl -s ifconfig.me || curl -s icanhazip.com || echo "")
    if [ -z "$IP_ADDRESS" ]; then
        read -p "Enter server IP address: " IP_ADDRESS
    else
        read -p "Server IP address [$IP_ADDRESS]: " input_ip
        if [ -n "$input_ip" ]; then
            IP_ADDRESS="$input_ip"
        fi
    fi

    print_success "Configuration gathered:"
    echo "  Domain: $DOMAIN"
    echo "  Email: $EMAIL"
    echo "  Hostname: $HOSTNAME"
    echo "  IP Address: $IP_ADDRESS"
}

# Clone or update mail-rs repository
setup_mail_rs() {
    print_info "Setting up mail-rs..."

    if [ -d "$INSTALL_DIR" ]; then
        print_warning "Installation directory exists, updating..."
        cd "$INSTALL_DIR"
        git pull
    else
        print_info "Cloning mail-rs repository..."
        # For now, assume we're installing from current directory
        mkdir -p "$INSTALL_DIR"
        cp -r "$(pwd)"/* "$INSTALL_DIR/"
    fi

    print_success "Mail-rs code ready at $INSTALL_DIR"
}

# Build mail-rs
build_mail_rs() {
    print_info "Building mail-rs..."

    cd "$INSTALL_DIR/mail-rs"

    # Source cargo env if needed
    if [ -f "$HOME/.cargo/env" ]; then
        . "$HOME/.cargo/env"
    fi

    cargo build --release

    print_success "Mail-rs built successfully"
}

# Create directories
create_directories() {
    print_info "Creating directories..."

    mkdir -p "$DATA_DIR"
    mkdir -p "$BACKUP_DIR"
    mkdir -p "$LOG_DIR"
    mkdir -p "$INSTALL_DIR/certs"
    mkdir -p "$INSTALL_DIR/test_data/dkim"

    chmod 750 "$DATA_DIR"
    chmod 750 "$BACKUP_DIR"
    chmod 755 "$LOG_DIR"

    print_success "Directories created"
}

# Generate DKIM keys
generate_dkim_keys() {
    print_info "Generating DKIM keys..."

    DKIM_PRIVATE="$INSTALL_DIR/test_data/dkim/dkim_private.pem"
    DKIM_PUBLIC="$INSTALL_DIR/test_data/dkim/dkim_public.pem"

    if [ -f "$DKIM_PRIVATE" ] && [ -f "$DKIM_PUBLIC" ]; then
        print_warning "DKIM keys already exist, skipping generation"
        return
    fi

    openssl genrsa -out "$DKIM_PRIVATE" 2048
    openssl rsa -in "$DKIM_PRIVATE" -pubout -out "$DKIM_PUBLIC"

    chmod 600 "$DKIM_PRIVATE"
    chmod 644 "$DKIM_PUBLIC"

    print_success "DKIM keys generated"
}

# Create configuration file
create_config() {
    print_info "Creating configuration file..."

    CONFIG_FILE="$INSTALL_DIR/mail-rs/config.toml"

    cat > "$CONFIG_FILE" << EOF
# Mail-rs Configuration
# Generated by installation script

[server]
hostname = "$HOSTNAME"
domain = "$DOMAIN"

[smtp]
host = "0.0.0.0"
port = 25
submission_port = 587
max_message_size = 26214400  # 25 MB
timeout_seconds = 300

[imap]
host = "0.0.0.0"
port = 143
tls_port = 993

[storage]
maildir_path = "$DATA_DIR"

[security]
enable_tls = true
cert_path = "$INSTALL_DIR/certs/server.crt"
key_path = "$INSTALL_DIR/certs/server.key"
require_tls = false

[authentication]
spf_enabled = true
spf_reject_on_fail = false
dkim_enabled = true
dkim_domain = "$DOMAIN"
dkim_selector = "default"
dkim_private_key_path = "$INSTALL_DIR/test_data/dkim/dkim_private.pem"
dkim_validate_incoming = true

[quotas]
enabled = true
default_storage_mb = 1024
default_daily_messages = 100
max_message_size_mb = 25

[antispam.greylist]
enabled = true
delay_seconds = 300
auto_whitelist_after_days = 7
cleanup_after_days = 30

[api]
enabled = true
host = "127.0.0.1"
port = 8080
admin_username = "admin"
# TODO: Change default password!
admin_password_hash = "\$2b\$12\$LQv3c1yqBWVHxkd0LHAkCOYz6TtxMQJqhN8/LewY5GyYwYvUVZmTu"  # "admin"
EOF

    print_success "Configuration file created: $CONFIG_FILE"
    print_warning "IMPORTANT: Change the default admin password!"
}

# Create systemd service
create_systemd_service() {
    print_info "Creating systemd service..."

    SERVICE_FILE="$SYSTEMD_DIR/mail-rs.service"

    cat > "$SERVICE_FILE" << EOF
[Unit]
Description=Mail-rs Mail Server
After=network.target

[Service]
Type=simple
User=root
WorkingDirectory=$INSTALL_DIR/mail-rs
ExecStart=$INSTALL_DIR/mail-rs/target/release/mail-rs
Restart=always
RestartSec=10
StandardOutput=append:$LOG_DIR/mail-rs.log
StandardError=append:$LOG_DIR/mail-rs-error.log

[Install]
WantedBy=multi-user.target
EOF

    systemctl daemon-reload
    systemctl enable mail-rs

    print_success "Systemd service created and enabled"
}

# Generate SSL certificates
generate_ssl_certificates() {
    print_info "Setting up SSL certificates..."

    # Check if certbot is available
    if ! command -v certbot &> /dev/null; then
        print_warning "Certbot not found, skipping Let's Encrypt setup"
        print_info "Generating self-signed certificate..."

        openssl req -x509 -newkey rsa:4096 -nodes \
            -keyout "$INSTALL_DIR/certs/server.key" \
            -out "$INSTALL_DIR/certs/server.crt" \
            -days 365 \
            -subj "/C=US/ST=State/L=City/O=Organization/CN=$HOSTNAME"

        print_warning "Self-signed certificate created"
        print_warning "For production, obtain a real certificate from Let's Encrypt"
        return
    fi

    # Try to obtain Let's Encrypt certificate
    print_info "Requesting Let's Encrypt certificate..."
    print_warning "This requires ports 80/443 to be open and DNS configured"

    read -p "Attempt Let's Encrypt certificate? (y/N): " attempt_le
    if [ "$attempt_le" = "y" ] || [ "$attempt_le" = "Y" ]; then
        certbot certonly --standalone --non-interactive --agree-tos \
            -d "$HOSTNAME" -m "$EMAIL" || {
            print_warning "Let's Encrypt failed, using self-signed certificate"
            openssl req -x509 -newkey rsa:4096 -nodes \
                -keyout "$INSTALL_DIR/certs/server.key" \
                -out "$INSTALL_DIR/certs/server.crt" \
                -days 365 \
                -subj "/C=US/ST=State/L=City/O=Organization/CN=$HOSTNAME"
        }

        # Copy Let's Encrypt certificates if successful
        if [ -f "/etc/letsencrypt/live/$HOSTNAME/fullchain.pem" ]; then
            cp "/etc/letsencrypt/live/$HOSTNAME/fullchain.pem" "$INSTALL_DIR/certs/server.crt"
            cp "/etc/letsencrypt/live/$HOSTNAME/privkey.pem" "$INSTALL_DIR/certs/server.key"
            print_success "Let's Encrypt certificate installed"
        fi
    else
        print_info "Generating self-signed certificate..."
        openssl req -x509 -newkey rsa:4096 -nodes \
            -keyout "$INSTALL_DIR/certs/server.key" \
            -out "$INSTALL_DIR/certs/server.crt" \
            -days 365 \
            -subj "/C=US/ST=State/L=City/O=Organization/CN=$HOSTNAME"
        print_success "Self-signed certificate created"
    fi
}

# Display DNS configuration
display_dns_config() {
    print_info "Generating DNS configuration..."

    # Extract DKIM public key
    DKIM_PUBLIC_KEY=$(grep -v "BEGIN\|END\|PUBLIC KEY" "$INSTALL_DIR/test_data/dkim/dkim_public.pem" | tr -d '\n')

    echo ""
    echo "========================================"
    echo "  DNS CONFIGURATION REQUIRED"
    echo "========================================"
    echo ""
    echo "Add the following DNS records to $DOMAIN:"
    echo ""
    echo "1. A Record (Mail Server):"
    echo "   Name: $HOSTNAME"
    echo "   Type: A"
    echo "   Value: $IP_ADDRESS"
    echo "   TTL: 3600"
    echo ""
    echo "2. MX Record:"
    echo "   Name: $DOMAIN"
    echo "   Type: MX"
    echo "   Priority: 10"
    echo "   Value: $HOSTNAME."
    echo "   TTL: 3600"
    echo ""
    echo "3. SPF Record:"
    echo "   Name: $DOMAIN"
    echo "   Type: TXT"
    echo "   Value: \"v=spf1 mx a:$HOSTNAME -all\""
    echo "   TTL: 3600"
    echo ""
    echo "4. DKIM Record:"
    echo "   Name: default._domainkey.$DOMAIN"
    echo "   Type: TXT"
    echo "   Value: \"v=DKIM1; k=rsa; p=$DKIM_PUBLIC_KEY\""
    echo "   TTL: 3600"
    echo ""
    echo "5. DMARC Record:"
    echo "   Name: _dmarc.$DOMAIN"
    echo "   Type: TXT"
    echo "   Value: \"v=DMARC1; p=quarantine; rua=mailto:postmaster@$DOMAIN; pct=100; adkim=s; aspf=s\""
    echo "   TTL: 3600"
    echo ""
    echo "6. Autodiscover (optional):"
    echo "   Name: autoconfig.$DOMAIN"
    echo "   Type: CNAME"
    echo "   Value: $HOSTNAME."
    echo ""
    echo "   Name: autodiscover.$DOMAIN"
    echo "   Type: CNAME"
    echo "   Value: $HOSTNAME."
    echo ""
    echo "========================================"
    echo "  IMPORTANT NOTES"
    echo "========================================"
    echo ""
    echo "- DNS changes may take up to 48 hours to propagate"
    echo "- Test your configuration at: https://mxtoolbox.com/"
    echo "- Verify SPF: dig TXT $DOMAIN"
    echo "- Verify DKIM: dig TXT default._domainkey.$DOMAIN"
    echo "- Verify DMARC: dig TXT _dmarc.$DOMAIN"
    echo ""

    # Save DNS config to file
    DNS_FILE="$INSTALL_DIR/DNS_CONFIGURATION.txt"
    {
        echo "DNS Configuration for $DOMAIN"
        echo "Generated: $(date)"
        echo ""
        echo "$HOSTNAME.	3600	IN	A	$IP_ADDRESS"
        echo "$DOMAIN.	3600	IN	MX	10 $HOSTNAME."
        echo "$DOMAIN.	3600	IN	TXT	\"v=spf1 mx a:$HOSTNAME -all\""
        echo "default._domainkey.$DOMAIN.	3600	IN	TXT	\"v=DKIM1; k=rsa; p=$DKIM_PUBLIC_KEY\""
        echo "_dmarc.$DOMAIN.	3600	IN	TXT	\"v=DMARC1; p=quarantine; rua=mailto:postmaster@$DOMAIN; pct=100; adkim=s; aspf=s\""
    } > "$DNS_FILE"

    print_success "DNS configuration saved to: $DNS_FILE"
}

# Configure firewall
configure_firewall() {
    print_info "Configuring firewall..."

    # Detect firewall
    if command -v ufw &> /dev/null; then
        # UFW (Ubuntu/Debian)
        ufw allow 25/tcp    # SMTP
        ufw allow 587/tcp   # Submission
        ufw allow 143/tcp   # IMAP
        ufw allow 993/tcp   # IMAPS
        ufw allow 80/tcp    # HTTP (for Let's Encrypt)
        ufw allow 443/tcp   # HTTPS
        print_success "Firewall configured (ufw)"

    elif command -v firewall-cmd &> /dev/null; then
        # FirewallD (Fedora/RHEL)
        firewall-cmd --permanent --add-service=smtp
        firewall-cmd --permanent --add-service=smtp-submission
        firewall-cmd --permanent --add-service=imap
        firewall-cmd --permanent --add-service=imaps
        firewall-cmd --permanent --add-service=http
        firewall-cmd --permanent --add-service=https
        firewall-cmd --reload
        print_success "Firewall configured (firewalld)"

    else
        print_warning "No supported firewall found"
        print_warning "Please manually open ports: 25, 587, 143, 993, 80, 443"
    fi
}

# Start mail server
start_server() {
    print_info "Starting mail-rs service..."

    systemctl start mail-rs
    sleep 2

    if systemctl is-active --quiet mail-rs; then
        print_success "Mail-rs is running!"
    else
        print_error "Mail-rs failed to start"
        print_info "Check logs: journalctl -u mail-rs -f"
        exit 1
    fi
}

# Display final instructions
display_final_instructions() {
    echo ""
    echo "========================================"
    echo "  INSTALLATION COMPLETE!"
    echo "========================================"
    echo ""
    echo "Mail-rs has been successfully installed and started."
    echo ""
    echo "Configuration:"
    echo "  - Install directory: $INSTALL_DIR"
    echo "  - Data directory: $DATA_DIR"
    echo "  - Backup directory: $BACKUP_DIR"
    echo "  - Log directory: $LOG_DIR"
    echo "  - Config file: $INSTALL_DIR/mail-rs/config.toml"
    echo ""
    echo "Service Management:"
    echo "  - Start: systemctl start mail-rs"
    echo "  - Stop: systemctl stop mail-rs"
    echo "  - Restart: systemctl restart mail-rs"
    echo "  - Status: systemctl status mail-rs"
    echo "  - Logs: journalctl -u mail-rs -f"
    echo ""
    echo "Next Steps:"
    echo "  1. Configure DNS records (see $DNS_FILE)"
    echo "  2. Wait for DNS propagation (24-48 hours)"
    echo "  3. Test email sending/receiving"
    echo "  4. Change default admin password in config.toml"
    echo "  5. Set up automatic backups (cron)"
    echo ""
    echo "Web Admin Interface:"
    echo "  - URL: http://localhost:8080"
    echo "  - Username: admin"
    echo "  - Password: admin (CHANGE THIS!)"
    echo ""
    echo "Documentation: $INSTALL_DIR/docs/"
    echo ""
    print_success "Happy mailing!"
    echo ""
}

# Main installation flow
main() {
    print_header
    check_root
    check_system_requirements
    install_dependencies
    install_rust
    gather_configuration
    setup_mail_rs
    build_mail_rs
    create_directories
    generate_dkim_keys
    create_config
    create_systemd_service
    generate_ssl_certificates
    configure_firewall
    display_dns_config
    start_server
    display_final_instructions
}

# Run main installation
main
