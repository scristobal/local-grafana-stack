#!/bin/bash

set -e

GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m' # No Color

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
K6_SCRIPTS_DIR="$SCRIPT_DIR/k6"

TESTS=(
  "basic-load:Standard load test (4min, 20 users)"
  "stress-test:High load stress test (10min, 150 users)"
  "spike-test:Sudden traffic spikes (4min, 300 peak)"
  "soak-test:Long-running stability test (13min, 20 users)"
  "error-test:Error handling test (3min, 10 users)"
  "profiling-test:CPU profiling test (3min, 5 users)"
)

show_usage() {
  echo -e "${GREEN}K6 Load Test Runner${NC}"
  echo ""
  echo "Usage: $0 [test-name] [k6-options]"
  echo ""
  echo "Available tests:"
  for test in "${TESTS[@]}"; do
    name="${test%%:*}"
    desc="${test#*:}"
    echo -e "  ${YELLOW}$name${NC}"
    echo -e "    $desc"
  done
  echo ""
  echo "Options:"
  echo "  --list              List all available tests"
  echo "  --help              Show this help message"
  echo ""
  echo "Examples:"
  echo "  $0 basic-load"
  echo "  $0 stress-test --summary-export=results.json"
  echo "  $0 basic-load --vus 50 --duration 30s"
  echo ""
}

list_tests() {
  echo -e "${GREEN}Available K6 Tests:${NC}"
  echo ""
  for test in "${TESTS[@]}"; do
    name="${test%%:*}"
    desc="${test#*:}"
    script_path="$K6_SCRIPTS_DIR/${name}.js"
    if [ -f "$script_path" ]; then
      echo -e "  ${GREEN}✓${NC} ${YELLOW}$name${NC} - $desc"
    else
      echo -e "  ${RED}✗${NC} ${YELLOW}$name${NC} - Script not found"
    fi
  done
  echo ""
}

if [ $# -eq 0 ] || [ "$1" = "--help" ]; then
  show_usage
  exit 0
fi

if [ "$1" = "--list" ]; then
  list_tests
  exit 0
fi

TEST_NAME="$1"
shift

SCRIPT_PATH="$K6_SCRIPTS_DIR/${TEST_NAME}.js"
if [ ! -f "$SCRIPT_PATH" ]; then
  echo -e "${RED}Error: Test '$TEST_NAME' not found${NC}"
  echo ""
  list_tests
  exit 1
fi

if ! docker info > /dev/null 2>&1; then
  echo -e "${RED}Error: Docker is not running${NC}"
  exit 1
fi

if ! docker-compose ps rust-app | grep -q "Up"; then
  echo -e "${YELLOW}Warning: rust-app is not running${NC}"
  echo "Starting rust-app..."
  docker-compose up -d rust-app
  echo "Waiting for rust-app to be ready..."
  sleep 5
fi

echo -e "${GREEN}Running K6 test: ${YELLOW}$TEST_NAME${NC}"
echo -e "Script: $SCRIPT_PATH"
echo -e "Additional k6 options: $*"
echo ""
echo "Press Ctrl+C to stop the test"
echo ""

docker-compose run --rm k6 run "/scripts/${TEST_NAME}.js" "$@"

EXIT_CODE=$?

echo ""

if [ $EXIT_CODE -eq 0 ]; then
  echo -e "${GREEN}✓ Test completed successfully${NC}"
else
  echo -e "${RED}✗ Test failed with exit code $EXIT_CODE${NC}"
fi

exit $EXIT_CODE
