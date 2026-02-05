#!/bin/bash
# =============================================================================
# RFQ Arena - Start Script
# =============================================================================
# Starts the frontend (Next.js) and backend services (RFQ Domain + Aomi Agent)
#
# Usage:
#   ./start.sh                              # Defaults: rfq=3335, aomi=8080
#   ./start.sh --rfq-port 3335 --aomi-port 8080
#   ./start.sh --help
#
# Prerequisites:
#   - Node.js & npm (for frontend)
#   - Rust & Cargo (for RFQ domain server)
#   - ANTHROPIC_API_KEY environment variable (for LLM compiler)
# =============================================================================

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Default ports
RFQ_PORT=3335
AOMI_PORT=8080
FE_PORT=3000

# Script directory
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# PIDs for cleanup
RFQ_PID=""
AOMI_PID=""
FE_PID=""

# =============================================================================
# Functions
# =============================================================================

usage() {
    echo "Usage: $0 [OPTIONS]"
    echo ""
    echo "Options:"
    echo "  --rfq-port PORT    Port for RFQ Domain server (default: $RFQ_PORT)"
    echo "  --aomi-port PORT   Port for Aomi Agent server (default: $AOMI_PORT)"
    echo "  --fe-port PORT     Port for Frontend dev server (default: $FE_PORT)"
    echo "  --no-fe            Skip starting the frontend"
    echo "  --no-aomi          Skip starting the Aomi agent (if not available)"
    echo "  --mock             Run RFQ server in mock mode (no Delta testnet)"
    echo "  -h, --help         Show this help message"
    echo ""
    echo "Environment variables:"
    echo "  ANTHROPIC_API_KEY  Required for LLM quote compilation"
    echo ""
    echo "Examples:"
    echo "  $0                                    # Start all services with defaults"
    echo "  $0 --rfq-port 3335 --aomi-port 8080  # Custom ports"
    echo "  $0 --mock --no-aomi                  # RFQ only, mock mode"
    exit 0
}

log_info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

log_success() {
    echo -e "${GREEN}[OK]${NC} $1"
}

log_warn() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

cleanup() {
    echo ""
    log_info "Shutting down services..."
    
    if [ -n "$FE_PID" ] && kill -0 "$FE_PID" 2>/dev/null; then
        log_info "Stopping frontend (PID: $FE_PID)..."
        kill "$FE_PID" 2>/dev/null || true
    fi
    
    if [ -n "$AOMI_PID" ] && kill -0 "$AOMI_PID" 2>/dev/null; then
        log_info "Stopping Aomi agent (PID: $AOMI_PID)..."
        kill "$AOMI_PID" 2>/dev/null || true
    fi
    
    if [ -n "$RFQ_PID" ] && kill -0 "$RFQ_PID" 2>/dev/null; then
        log_info "Stopping RFQ server (PID: $RFQ_PID)..."
        kill "$RFQ_PID" 2>/dev/null || true
    fi
    
    # Also clean up any orphaned processes
    pkill -f "rfq-domain" 2>/dev/null || true
    pkill -f "next dev" 2>/dev/null || true
    
    log_success "All services stopped."
    exit 0
}

check_port() {
    local port=$1
    if lsof -Pi :$port -sTCP:LISTEN -t >/dev/null 2>&1; then
        return 0  # Port is in use
    else
        return 1  # Port is free
    fi
}

wait_for_service() {
    local url=$1
    local name=$2
    local max_attempts=${3:-30}
    local attempt=1
    
    while [ $attempt -le $max_attempts ]; do
        if curl -s "$url" >/dev/null 2>&1; then
            return 0
        fi
        sleep 1
        attempt=$((attempt + 1))
    done
    return 1
}

# =============================================================================
# Parse Arguments
# =============================================================================

MOCK_MODE=""
SKIP_FE=false
SKIP_AOMI=false

while [[ $# -gt 0 ]]; do
    case $1 in
        --rfq-port)
            RFQ_PORT="$2"
            shift 2
            ;;
        --aomi-port)
            AOMI_PORT="$2"
            shift 2
            ;;
        --fe-port)
            FE_PORT="$2"
            shift 2
            ;;
        --no-fe)
            SKIP_FE=true
            shift
            ;;
        --no-aomi)
            SKIP_AOMI=true
            shift
            ;;
        --mock)
            MOCK_MODE="--mock"
            shift
            ;;
        -h|--help)
            usage
            ;;
        *)
            log_error "Unknown option: $1"
            usage
            ;;
    esac
done

# =============================================================================
# Pre-flight Checks
# =============================================================================

trap cleanup EXIT INT TERM

echo ""
echo "=========================================="
echo "  RFQ Arena - Development Server"
echo "=========================================="
echo ""

# Check for ANTHROPIC_API_KEY
if [ -z "$ANTHROPIC_API_KEY" ]; then
    log_warn "ANTHROPIC_API_KEY not set. LLM quote compilation will fail."
    log_warn "Set it with: export ANTHROPIC_API_KEY=your_key"
fi

# Check ports
if check_port $RFQ_PORT; then
    log_error "Port $RFQ_PORT is already in use (RFQ server)"
    exit 1
fi

if [ "$SKIP_AOMI" = false ] && check_port $AOMI_PORT; then
    log_error "Port $AOMI_PORT is already in use (Aomi agent)"
    exit 1
fi

if [ "$SKIP_FE" = false ] && check_port $FE_PORT; then
    log_error "Port $FE_PORT is already in use (Frontend)"
    exit 1
fi

log_success "Port checks passed"

# =============================================================================
# Build RFQ Domain Server
# =============================================================================

log_info "Building RFQ Domain server..."
cd "$SCRIPT_DIR"

if ! cargo build -p rfq-domain 2>&1 | tail -5; then
    log_error "Failed to build RFQ Domain server"
    exit 1
fi
log_success "RFQ Domain server built"

# =============================================================================
# Start Services
# =============================================================================

# Start RFQ Domain Server
log_info "Starting RFQ Domain server on port $RFQ_PORT..."
cd "$SCRIPT_DIR/crates/domain"
cargo run -- --port "$RFQ_PORT" $MOCK_MODE > /tmp/rfq-domain.log 2>&1 &
RFQ_PID=$!
cd "$SCRIPT_DIR"

# Wait for RFQ server
if wait_for_service "http://localhost:$RFQ_PORT/health" "RFQ Domain" 30; then
    log_success "RFQ Domain server started (PID: $RFQ_PID)"
else
    log_error "RFQ Domain server failed to start. Check /tmp/rfq-domain.log"
    cat /tmp/rfq-domain.log | tail -20
    exit 1
fi

# Start Aomi Agent (if available and not skipped)
if [ "$SKIP_AOMI" = false ]; then
    # Check if aomi binary or server exists
    if command -v aomi &> /dev/null; then
        log_info "Starting Aomi Agent on port $AOMI_PORT..."
        aomi serve --port "$AOMI_PORT" > /tmp/aomi-agent.log 2>&1 &
        AOMI_PID=$!
        
        if wait_for_service "http://localhost:$AOMI_PORT/health" "Aomi Agent" 30; then
            log_success "Aomi Agent started (PID: $AOMI_PID)"
        else
            log_warn "Aomi Agent failed to start. Frontend agent features may not work."
            log_warn "Check /tmp/aomi-agent.log for details"
        fi
    else
        log_warn "Aomi CLI not found. Skipping agent server."
        log_warn "Install with: cargo install aomi-cli"
        SKIP_AOMI=true
    fi
fi

# Start Frontend
if [ "$SKIP_FE" = false ]; then
    log_info "Starting Frontend dev server on port $FE_PORT..."
    cd "$SCRIPT_DIR/web"
    
    # Install deps if needed
    if [ ! -d "node_modules" ]; then
        log_info "Installing frontend dependencies..."
        npm install
    fi
    
    # Set environment variables for the frontend
    export NEXT_PUBLIC_API_URL="http://localhost:$RFQ_PORT"
    export NEXT_PUBLIC_BACKEND_URL="http://localhost:$AOMI_PORT"
    
    npm run dev -- -p "$FE_PORT" > /tmp/frontend.log 2>&1 &
    FE_PID=$!
    cd "$SCRIPT_DIR"
    
    # Wait for frontend (Next.js takes longer)
    if wait_for_service "http://localhost:$FE_PORT" "Frontend" 60; then
        log_success "Frontend started (PID: $FE_PID)"
    else
        log_warn "Frontend may still be compiling. Check /tmp/frontend.log"
    fi
fi

# =============================================================================
# Summary
# =============================================================================

echo ""
echo "=========================================="
echo "  Services Running"
echo "=========================================="
echo ""
echo "  RFQ Domain Server:  http://localhost:$RFQ_PORT"
echo "    - Health:         http://localhost:$RFQ_PORT/health"
echo "    - Quotes:         http://localhost:$RFQ_PORT/quotes"
echo ""

if [ "$SKIP_AOMI" = false ] && [ -n "$AOMI_PID" ]; then
    echo "  Aomi Agent:         http://localhost:$AOMI_PORT"
    echo ""
fi

if [ "$SKIP_FE" = false ]; then
    echo "  Frontend:           http://localhost:$FE_PORT"
    echo ""
fi

echo "  Logs:"
echo "    - RFQ:      /tmp/rfq-domain.log"
[ "$SKIP_AOMI" = false ] && echo "    - Aomi:     /tmp/aomi-agent.log"
[ "$SKIP_FE" = false ] && echo "    - Frontend: /tmp/frontend.log"
echo ""
echo "  Press Ctrl+C to stop all services"
echo "=========================================="
echo ""

# Keep script running and forward signals
wait
