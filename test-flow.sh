#!/bin/bash
# =============================================================================
# RFQ Arena - API Flow Test Script
# =============================================================================
# Tests the complete flow: Create Quote -> Fill Quote -> ZK Proof -> Settlement
#
# Prerequisites:
#   - ANTHROPIC_API_KEY environment variable set
#   - RFQ Domain server running (./start.sh --mock --no-fe --no-aomi)
#
# Usage:
#   ./test-flow.sh                    # Run against localhost:3335
#   ./test-flow.sh --port 3337        # Custom port
#   ./test-flow.sh --testnet          # Run against testnet (no --mock)
# =============================================================================

set -e

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
NC='\033[0m'

# Defaults
PORT=3335
BASE_URL="http://localhost:$PORT"
MOCK_MODE=true

# Parse args
while [[ $# -gt 0 ]]; do
    case $1 in
        --port)
            PORT="$2"
            BASE_URL="http://localhost:$PORT"
            shift 2
            ;;
        --testnet)
            MOCK_MODE=false
            shift
            ;;
        *)
            echo "Unknown option: $1"
            exit 1
            ;;
    esac
done

echo ""
echo -e "${CYAN}=========================================="
echo "  RFQ Arena - API Flow Test"
echo -e "==========================================${NC}"
echo ""
echo -e "  Server: ${BLUE}$BASE_URL${NC}"
echo -e "  Mode:   ${YELLOW}$([ "$MOCK_MODE" = true ] && echo "Mock" || echo "Testnet")${NC}"
echo ""

# -----------------------------------------------------------------------------
# Step 0: Health Check
# -----------------------------------------------------------------------------
echo -e "${BLUE}[Step 0]${NC} Health check..."
HEALTH=$(curl -s "$BASE_URL/health")
echo "$HEALTH" | jq .

if ! echo "$HEALTH" | jq -e '.status == "ok"' > /dev/null; then
    echo -e "${RED}[ERROR]${NC} Server not healthy. Start it with:"
    echo "  ANTHROPIC_API_KEY=... cargo run -p rfq-domain -- --mock --port $PORT"
    exit 1
fi
echo -e "${GREEN}[OK]${NC} Server is healthy"
echo ""

# -----------------------------------------------------------------------------
# Step 1: Create a Quote (Maker)
# -----------------------------------------------------------------------------
echo -e "${BLUE}[Step 1]${NC} Creating quote as Maker..."
echo -e "  Quote: ${CYAN}Buy up to 1 dETH at most 2000 USDD, expires in 10 minutes${NC}"
echo ""

# Generate a maker ID (in production, this would be a real wallet address)
MAKER_ID="maker_$(date +%s)"
MAKER_SHARD=9

CREATE_RESPONSE=$(curl -s -X POST "$BASE_URL/quotes" \
    -H "Content-Type: application/json" \
    -d "{
        \"text\": \"Buy up to 1 dETH at most 2000 USDD, expires in 10 minutes. Only accept feeds from FeedA or FeedB, require 2 sources within 0.5% of each other, and feeds must be less than 5 seconds old.\",
        \"maker_owner_id\": \"$MAKER_ID\",
        \"maker_shard\": $MAKER_SHARD
    }")

echo "Response:"
echo "$CREATE_RESPONSE" | jq .

# Handle both nested (.quote.id) and flat (.id) response formats
QUOTE_ID=$(echo "$CREATE_RESPONSE" | jq -r '.quote.id // .id // empty')

if [ -z "$QUOTE_ID" ]; then
    echo -e "${RED}[ERROR]${NC} Failed to create quote"
    echo "$CREATE_RESPONSE"
    exit 1
fi

echo -e "${GREEN}[OK]${NC} Quote created: $QUOTE_ID"
echo ""

# -----------------------------------------------------------------------------
# Step 2: List Quotes (Taker discovers the market)
# -----------------------------------------------------------------------------
echo -e "${BLUE}[Step 2]${NC} Listing active quotes..."

QUOTES=$(curl -s "$BASE_URL/quotes")
echo "Active quotes:"
echo "$QUOTES" | jq '.[] | {id, status, asset: .spec.asset, side: .spec.side}'
echo ""

# -----------------------------------------------------------------------------
# Step 3: Get Quote Details (Taker evaluates)
# -----------------------------------------------------------------------------
echo -e "${BLUE}[Step 3]${NC} Getting quote details..."

QUOTE_DETAIL=$(curl -s "$BASE_URL/quotes/$QUOTE_ID")
echo "Quote constraints:"
echo "$QUOTE_DETAIL" | jq '.constraints'
echo ""

# -----------------------------------------------------------------------------
# Step 4: Attempt to Fill the Quote (Taker)
# -----------------------------------------------------------------------------
echo -e "${BLUE}[Step 4]${NC} Filling quote as Taker..."

TAKER_ID="taker_$(date +%s)"
TAKER_SHARD=9
NOW=$(date +%s)

# Prepare feed evidence (simulating 2 price feeds)
FEED_EVIDENCE="[
    {
        \"source\": \"FeedA\",
        \"asset\": \"dETH\",
        \"price\": 1950.0,
        \"timestamp\": $NOW,
        \"signature\": \"sig_feed_a_$(date +%s)\"
    },
    {
        \"source\": \"FeedB\",
        \"asset\": \"dETH\",
        \"price\": 1951.0,
        \"timestamp\": $NOW,
        \"signature\": \"sig_feed_b_$(date +%s)\"
    }
]"

echo -e "  Taker: ${CYAN}$TAKER_ID${NC}"
echo -e "  Size:  ${CYAN}1.0 dETH${NC}"
echo -e "  Price: ${CYAN}1950.5 USDD${NC} (average of feeds)"
echo ""

FILL_RESPONSE=$(curl -s -X POST "$BASE_URL/quotes/$QUOTE_ID/fill" \
    -H "Content-Type: application/json" \
    -d "{
        \"taker_owner_id\": \"$TAKER_ID\",
        \"taker_shard\": $TAKER_SHARD,
        \"size\": 1.0,
        \"price\": 1950.5,
        \"feed_evidence\": $FEED_EVIDENCE
    }")

echo "Fill response:"
echo "$FILL_RESPONSE" | jq .

# Handle both response formats
FILL_SUCCESS=$(echo "$FILL_RESPONSE" | jq -r '.success // .result.accepted // false')
SDL_HASH=$(echo "$FILL_RESPONSE" | jq -r '.proof.sdl_hash // .result.sdl_hash // "none"')

if [ "$FILL_SUCCESS" = "true" ]; then
    echo -e "${GREEN}[OK]${NC} Fill accepted!"
    echo -e "  SDL Hash: ${CYAN}$SDL_HASH${NC}"
    echo ""
    
    # Show settlement details
    echo "Settlement details:"
    echo "$FILL_RESPONSE" | jq '.receipt.settlement // .result.settlement'
else
    echo -e "${YELLOW}[REJECTED]${NC} Fill was rejected"
    echo "Reason:"
    echo "$FILL_RESPONSE" | jq '.error // .result.reason'
fi
echo ""

# -----------------------------------------------------------------------------
# Step 5: Get Receipts (Audit trail)
# -----------------------------------------------------------------------------
echo -e "${BLUE}[Step 5]${NC} Getting fill receipts..."

RECEIPTS=$(curl -s "$BASE_URL/quotes/$QUOTE_ID/receipts")
echo "Receipts:"
echo "$RECEIPTS" | jq '.[] | {fill_id, accepted, timestamp: .attempted_at}'
echo ""

# -----------------------------------------------------------------------------
# Summary
# -----------------------------------------------------------------------------
echo -e "${CYAN}=========================================="
echo "  Summary"
echo -e "==========================================${NC}"
echo ""
echo -e "  Quote ID:      ${BLUE}$QUOTE_ID${NC}"
echo -e "  Maker:         ${GREEN}$MAKER_ID${NC}"
echo -e "  Taker:         ${GREEN}$TAKER_ID${NC}"
echo -e "  Fill Status:   $([ "$FILL_SUCCESS" = "true" ] && echo -e "${GREEN}ACCEPTED${NC}" || echo -e "${RED}REJECTED${NC}")"
if [ "$FILL_SUCCESS" = "true" ]; then
    echo -e "  SDL Hash:      ${CYAN}$SDL_HASH${NC}"
    echo ""
    echo -e "  ${GREEN}The fill was validated against Local Laws and a ZK proof was generated!${NC}"
    if [ "$MOCK_MODE" = true ]; then
        echo -e "  ${YELLOW}(Mock mode: proof not submitted to real testnet)${NC}"
    else
        echo -e "  ${GREEN}Proof submitted to Delta testnet for settlement.${NC}"
    fi
fi
echo ""
echo -e "${CYAN}==========================================${NC}"
echo ""
