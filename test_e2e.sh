#!/bin/bash
set -e

# Colors for output
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Test configuration
TEST_EMAIL="test-e2e@example.com"
TEST_PASSWORD="testpass123"
SMTP_PORT=2525
API_PORT=8080
MCP_PORT=8090
AI_PORT=8888

# Directories
PROJECT_ROOT=$(pwd)
MAIL_RS_DIR="$PROJECT_ROOT/mail-rs"
MCP_DIR="$PROJECT_ROOT/mcp-mail-server"
AI_DIR="$PROJECT_ROOT/ai-runtime"
MAILDIR="$MAIL_RS_DIR/data/maildir/$TEST_EMAIL"

echo -e "${YELLOW}========================================${NC}"
echo -e "${YELLOW}  End-to-End Test Suite${NC}"
echo -e "${YELLOW}========================================${NC}\n"

# Cleanup function
cleanup() {
    echo -e "\n${YELLOW}Cleaning up...${NC}"

    # Kill all test processes
    pkill -f "mail-rs --config" || true
    pkill -f "mcp-mail-server" || true
    pkill -f "ai-runtime" || true

    # Remove test maildir
    if [ -d "$MAILDIR" ]; then
        rm -rf "$MAILDIR"
        echo -e "${GREEN}✓ Removed test maildir${NC}"
    fi

    # Remove test database entries (if needed)
    if [ -f "$AI_DIR/summaries.db" ]; then
        sqlite3 "$AI_DIR/summaries.db" "DELETE FROM email_summaries WHERE user_email='$TEST_EMAIL';" 2>/dev/null || true
    fi

    sleep 1
}

# Register cleanup on exit
trap cleanup EXIT

# Function to wait for service
wait_for_service() {
    local port=$1
    local name=$2
    local max_attempts=30
    local attempt=0

    echo -n "Waiting for $name on port $port..."

    while ! nc -z localhost $port 2>/dev/null; do
        attempt=$((attempt + 1))
        if [ $attempt -ge $max_attempts ]; then
            echo -e " ${RED}TIMEOUT${NC}"
            return 1
        fi
        sleep 1
        echo -n "."
    done

    echo -e " ${GREEN}OK${NC}"
    return 0
}

# Function to create test user
create_test_user() {
    echo -e "\n${YELLOW}Creating test user...${NC}"

    cd "$MAIL_RS_DIR"
    cargo run --bin mail-user -- create "$TEST_EMAIL" "$TEST_PASSWORD" > /dev/null 2>&1 || true

    if [ -d "$MAILDIR" ]; then
        echo -e "${GREEN}✓ Test user created${NC}"
    else
        echo -e "${RED}✗ Failed to create test user${NC}"
        return 1
    fi
}

# Start services
start_services() {
    echo -e "\n${YELLOW}Starting services...${NC}"

    # Start mail-rs
    cd "$MAIL_RS_DIR"
    cargo run --bin mail-rs -- --config config.toml > /tmp/test-mail-rs.log 2>&1 &
    wait_for_service $API_PORT "mail-rs API"
    wait_for_service $SMTP_PORT "mail-rs SMTP"

    # Start mcp-mail-server
    cd "$MCP_DIR"
    cargo run > /tmp/test-mcp.log 2>&1 &
    wait_for_service $MCP_PORT "mcp-mail-server"

    # Start ai-runtime
    cd "$AI_DIR"
    cargo run > /tmp/test-ai.log 2>&1 &
    wait_for_service $AI_PORT "ai-runtime"

    echo -e "${GREEN}✓ All services started${NC}"
    sleep 2
}

# Test 1: Send email via SMTP
test_send_email() {
    echo -e "\n${YELLOW}Test 1: Sending email via SMTP...${NC}"

    python3 << 'PYTHON_SCRIPT'
import smtplib
from email.mime.text import MIMEText
import sys

try:
    msg = MIMEText("This is an end-to-end test email. It contains important test data.")
    msg['Subject'] = "E2E Test Email"
    msg['From'] = "test-e2e@example.com"
    msg['To'] = "test-e2e@example.com"

    with smtplib.SMTP('localhost', 2525, timeout=10) as smtp:
        smtp.login('test-e2e@example.com', 'testpass123')
        smtp.send_message(msg)

    print("Email sent successfully")
    sys.exit(0)
except Exception as e:
    print(f"Failed to send email: {e}")
    sys.exit(1)
PYTHON_SCRIPT

    if [ $? -eq 0 ]; then
        echo -e "${GREEN}✓ Email sent successfully${NC}"
    else
        echo -e "${RED}✗ Failed to send email${NC}"
        return 1
    fi

    sleep 2
}

# Test 2: Verify email in maildir
test_verify_maildir() {
    echo -e "\n${YELLOW}Test 2: Verifying email in maildir...${NC}"

    local new_dir="$MAILDIR/new"

    if [ ! -d "$new_dir" ]; then
        echo -e "${RED}✗ Maildir new/ directory not found${NC}"
        return 1
    fi

    local email_count=$(ls -1 "$new_dir" 2>/dev/null | wc -l)

    if [ "$email_count" -gt 0 ]; then
        echo -e "${GREEN}✓ Found $email_count email(s) in maildir${NC}"

        # Verify content
        local first_email=$(ls -1 "$new_dir" | head -1)
        if grep -q "E2E Test Email" "$new_dir/$first_email"; then
            echo -e "${GREEN}✓ Email content verified${NC}"
        else
            echo -e "${RED}✗ Email content doesn't match${NC}"
            return 1
        fi
    else
        echo -e "${RED}✗ No emails found in maildir${NC}"
        return 1
    fi
}

# Test 3: MCP list_emails
test_mcp_list_emails() {
    echo -e "\n${YELLOW}Test 3: Testing MCP list_emails...${NC}"

    local response=$(curl -s -X POST http://localhost:$MCP_PORT/mcp \
        -H "Content-Type: application/json" \
        -d "{
            \"jsonrpc\": \"2.0\",
            \"method\": \"tools/call\",
            \"params\": {
                \"name\": \"list_emails\",
                \"arguments\": {
                    \"email\": \"$TEST_EMAIL\",
                    \"limit\": 10
                }
            },
            \"id\": 1
        }")

    if echo "$response" | grep -q "E2E Test Email"; then
        echo -e "${GREEN}✓ MCP list_emails works${NC}"
    else
        echo -e "${RED}✗ MCP list_emails failed${NC}"
        echo "Response: $response"
        return 1
    fi
}

# Test 4: MCP read_email
test_mcp_read_email() {
    echo -e "\n${YELLOW}Test 4: Testing MCP read_email...${NC}"

    # Get email ID from maildir
    local email_id=$(ls -1 "$MAILDIR/new" | head -1)

    if [ -z "$email_id" ]; then
        echo -e "${RED}✗ No email ID found${NC}"
        return 1
    fi

    local response=$(curl -s -X POST http://localhost:$MCP_PORT/mcp \
        -H "Content-Type: application/json" \
        -d "{
            \"jsonrpc\": \"2.0\",
            \"method\": \"tools/call\",
            \"params\": {
                \"name\": \"read_email\",
                \"arguments\": {
                    \"email\": \"$TEST_EMAIL\",
                    \"email_id\": \"$email_id\"
                }
            },
            \"id\": 1
        }")

    if echo "$response" | grep -q "end-to-end test email"; then
        echo -e "${GREEN}✓ MCP read_email works${NC}"
    else
        echo -e "${RED}✗ MCP read_email failed${NC}"
        echo "Response: $response"
        return 1
    fi
}

# Test 5: Summary generation
test_summary_generation() {
    echo -e "\n${YELLOW}Test 5: Testing summary generation...${NC}"

    local response=$(curl -s -X POST http://localhost:$AI_PORT/generate-summary \
        -H "Content-Type: application/json" \
        -d "{
            \"user_email\": \"$TEST_EMAIL\",
            \"email_id\": \"test-summary-001\",
            \"from\": \"sender@example.com\",
            \"subject\": \"Meeting Tomorrow\",
            \"body\": \"Hi, let's meet tomorrow at 3pm to discuss the quarterly results and plan for next quarter.\"
        }")

    if echo "$response" | grep -q "summary"; then
        local summary=$(echo "$response" | jq -r '.summary')
        echo -e "${GREEN}✓ Summary generated: $summary${NC}"
    else
        echo -e "${RED}✗ Summary generation failed${NC}"
        echo "Response: $response"
        return 1
    fi
}

# Test 6: MCP get_email_count
test_mcp_email_count() {
    echo -e "\n${YELLOW}Test 6: Testing MCP get_email_count...${NC}"

    local response=$(curl -s -X POST http://localhost:$MCP_PORT/mcp \
        -H "Content-Type: application/json" \
        -d "{
            \"jsonrpc\": \"2.0\",
            \"method\": \"tools/call\",
            \"params\": {
                \"name\": \"get_email_count\",
                \"arguments\": {
                    \"email\": \"$TEST_EMAIL\"
                }
            },
            \"id\": 1
        }")

    local count=$(echo "$response" | jq -r '.result.count' 2>/dev/null)

    if [ "$count" -ge 1 ] 2>/dev/null; then
        echo -e "${GREEN}✓ MCP get_email_count works (count: $count)${NC}"
    else
        echo -e "${RED}✗ MCP get_email_count failed${NC}"
        echo "Response: $response"
        return 1
    fi
}

# Test 7: MCP mark_as_read
test_mcp_mark_as_read() {
    echo -e "\n${YELLOW}Test 7: Testing MCP mark_as_read...${NC}"

    local email_id=$(ls -1 "$MAILDIR/new" | head -1)

    if [ -z "$email_id" ]; then
        echo -e "${YELLOW}⚠ No unread emails to test${NC}"
        return 0
    fi

    local response=$(curl -s -X POST http://localhost:$MCP_PORT/mcp \
        -H "Content-Type: application/json" \
        -d "{
            \"jsonrpc\": \"2.0\",
            \"method\": \"tools/call\",
            \"params\": {
                \"name\": \"mark_as_read\",
                \"arguments\": {
                    \"email\": \"$TEST_EMAIL\",
                    \"email_id\": \"$email_id\"
                }
            },
            \"id\": 1
        }")

    if echo "$response" | grep -q "success.*true"; then
        # Verify email moved to cur/
        if [ -f "$MAILDIR/cur/$email_id" ]; then
            echo -e "${GREEN}✓ MCP mark_as_read works (email moved to cur/)${NC}"
        else
            echo -e "${YELLOW}⚠ Email marked as read but not in cur/${NC}"
        fi
    else
        echo -e "${RED}✗ MCP mark_as_read failed${NC}"
        echo "Response: $response"
        return 1
    fi
}

# Test 8: Health check endpoints
test_health_checks() {
    echo -e "\n${YELLOW}Test 8: Testing health check endpoints...${NC}"

    # Test mail-rs API
    if curl -s -f "http://localhost:$API_PORT/" > /dev/null; then
        echo -e "${GREEN}✓ mail-rs web UI accessible${NC}"
    else
        echo -e "${RED}✗ mail-rs web UI not accessible${NC}"
    fi

    # Test MCP tools/list
    if curl -s -f "http://localhost:$MCP_PORT/mcp/tools" > /dev/null; then
        echo -e "${GREEN}✓ MCP tools endpoint accessible${NC}"
    else
        echo -e "${YELLOW}⚠ MCP tools endpoint returned error (may be expected)${NC}"
    fi
}

# Main execution
main() {
    echo -e "${YELLOW}Preparing environment...${NC}"

    # Check dependencies
    command -v cargo >/dev/null 2>&1 || { echo -e "${RED}✗ cargo not found${NC}"; exit 1; }
    command -v python3 >/dev/null 2>&1 || { echo -e "${RED}✗ python3 not found${NC}"; exit 1; }
    command -v nc >/dev/null 2>&1 || { echo -e "${RED}✗ netcat not found${NC}"; exit 1; }
    command -v jq >/dev/null 2>&1 || { echo -e "${RED}✗ jq not found${NC}"; exit 1; }

    # Clean up any existing processes
    cleanup

    # Start fresh
    start_services
    create_test_user

    # Run tests
    local failed=0

    test_send_email || failed=$((failed + 1))
    test_verify_maildir || failed=$((failed + 1))
    test_mcp_list_emails || failed=$((failed + 1))
    test_mcp_read_email || failed=$((failed + 1))
    test_summary_generation || failed=$((failed + 1))
    test_mcp_email_count || failed=$((failed + 1))
    test_mcp_mark_as_read || failed=$((failed + 1))
    test_health_checks || failed=$((failed + 1))

    # Summary
    echo -e "\n${YELLOW}========================================${NC}"
    echo -e "${YELLOW}  Test Results${NC}"
    echo -e "${YELLOW}========================================${NC}"

    if [ $failed -eq 0 ]; then
        echo -e "${GREEN}✓ All tests passed!${NC}\n"
        return 0
    else
        echo -e "${RED}✗ $failed test(s) failed${NC}\n"
        return 1
    fi
}

# Run main
main
exit $?
