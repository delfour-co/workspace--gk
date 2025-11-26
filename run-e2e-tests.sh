#!/bin/bash
# Run E2E Tests for GK Mail Suite

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo "🚀 GK Mail - E2E Test Runner"
echo "=============================="
echo ""

# Check if Docker is running
if ! docker info > /dev/null 2>&1; then
    echo -e "${RED}❌ Docker is not running${NC}"
    exit 1
fi

echo -e "${GREEN}✅ Docker is running${NC}"

# Check if services are up
echo ""
echo "📋 Checking Docker services..."

SERVICES=("gk-mail-rs-dev" "gk-mcp-mail-dev" "gk-ollama-dev" "gk-ai-runtime-dev" "gk-web-ui-dev")
ALL_UP=true

for service in "${SERVICES[@]}"; do
    if docker ps --format '{{.Names}}' | grep -q "^${service}$"; then
        echo -e "  ${GREEN}✅${NC} $service"
    else
        echo -e "  ${RED}❌${NC} $service (not running)"
        ALL_UP=false
    fi
done

if [ "$ALL_UP" = false ]; then
    echo ""
    echo -e "${YELLOW}⚠️  Some services are not running${NC}"
    echo "Starting services with docker-compose..."
    docker compose -f docker-compose.dev.yml up -d
    echo ""
    echo "Waiting 30 seconds for services to be ready..."
    sleep 30
fi

# Check if Ollama model is ready
echo ""
echo "🤖 Checking Ollama model..."
if docker exec gk-ollama-dev ollama list | grep -q "llama3.1:8b"; then
    echo -e "${GREEN}✅ llama3.1:8b model is ready${NC}"
else
    echo -e "${RED}❌ llama3.1:8b model not found${NC}"
    echo "Please ensure the model is pulled:"
    echo "  docker exec gk-ollama-dev ollama pull llama3.1:8b"
    exit 1
fi

# Parse command line arguments
TEST_FILTER=""
SHOW_OUTPUT="--nocapture"
TEST_THREADS="--test-threads=1"
VERBOSE=""

while [[ $# -gt 0 ]]; do
    case $1 in
        -t|--test)
            TEST_FILTER="--test $2"
            shift 2
            ;;
        -v|--verbose)
            VERBOSE="RUST_LOG=debug"
            shift
            ;;
        --parallel)
            TEST_THREADS=""
            shift
            ;;
        -h|--help)
            echo "Usage: ./run-e2e-tests.sh [OPTIONS]"
            echo ""
            echo "Options:"
            echo "  -t, --test <name>     Run specific test (e.g., e2e_test_1_send_email)"
            echo "  -v, --verbose         Enable debug logging"
            echo "  --parallel            Run tests in parallel (not recommended)"
            echo "  -h, --help            Show this help message"
            echo ""
            echo "Examples:"
            echo "  ./run-e2e-tests.sh                              # Run all tests"
            echo "  ./run-e2e-tests.sh -t e2e_test_1_send_email    # Run test 1"
            echo "  ./run-e2e-tests.sh -v                          # Run with debug logs"
            exit 0
            ;;
        *)
            echo "Unknown option: $1"
            echo "Use -h or --help for usage information"
            exit 1
            ;;
    esac
done

# Run tests
echo ""
echo "🧪 Running E2E Tests..."
echo "========================"
echo ""

if [ -n "$TEST_FILTER" ]; then
    echo "Running specific test: $TEST_FILTER"
else
    echo "Running all E2E tests"
fi

# Build command
CMD="cargo test"
if [ -n "$TEST_FILTER" ]; then
    CMD="$CMD $TEST_FILTER"
else
    CMD="$CMD --test 'e2e_*'"
fi

CMD="$CMD -- $TEST_THREADS $SHOW_OUTPUT"

if [ -n "$VERBOSE" ]; then
    CMD="$VERBOSE $CMD"
fi

echo ""
echo "Command: $CMD"
echo ""

# Run tests
if eval $CMD; then
    echo ""
    echo -e "${GREEN}✅ All tests passed!${NC}"
    echo ""
    exit 0
else
    echo ""
    echo -e "${RED}❌ Some tests failed${NC}"
    echo ""
    echo "Troubleshooting:"
    echo "  1. Check service logs:"
    echo "     docker logs gk-ai-runtime-dev --tail 50"
    echo "     docker logs gk-mail-rs-dev --tail 50"
    echo "  2. Check service health:"
    echo "     curl http://localhost:8888/health"
    echo "     curl http://localhost:8090/health"
    echo "  3. Restart services:"
    echo "     docker compose -f docker-compose.dev.yml restart"
    echo ""
    exit 1
fi
