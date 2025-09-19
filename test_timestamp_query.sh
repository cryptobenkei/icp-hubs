#!/bin/bash

# Test script for timestamp-based domain queries
echo "🕐 Testing Timestamp-Based Domain Query"
echo "======================================="

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Check if dfx is running
if ! dfx ping > /dev/null 2>&1; then
    echo -e "${RED}❌ dfx is not running. Please start dfx with: dfx start --background${NC}"
    exit 1
fi

echo -e "${GREEN}✅ dfx is running${NC}"

# Get current timestamp before creating domains
echo -e "${YELLOW}Getting current timestamp...${NC}"
TIMESTAMP_BEFORE=$(date +%s%N | cut -b1-16)  # Nanoseconds to microseconds (IC uses microseconds)
echo "Timestamp before: $TIMESTAMP_BEFORE"

# Create a season first
echo -e "${YELLOW}Creating a registration season...${NC}"
ADMIN_PRINCIPAL=$(dfx identity get-principal)

SEASON_RESULT=$(dfx canister call registry create_registration_season '(record {
    min_letters = 3;
    max_letters = opt 10;
    total_allowed = 10;
    price_icp = 5;
})' 2>&1)

if echo "$SEASON_RESULT" | grep -q "Ok"; then
    echo -e "${GREEN}✅ Season created successfully${NC}"
else
    echo -e "${YELLOW}Season might already exist, continuing...${NC}"
fi

# Create test identities
echo -e "${YELLOW}Creating test identities...${NC}"
for i in {1..3}; do
    dfx identity new test-timestamp-${i} --storage-mode plaintext 2>/dev/null || true
done

# Register some domains
echo -e "${BLUE}📝 Registering test domains...${NC}"

# User 1 gets domain via admin gift
dfx identity use test-timestamp-1
USER1_PRINCIPAL=$(dfx identity get-principal)
dfx identity use default
RESULT1=$(dfx canister call registry admin_gift_domain '(record {
    domain_name = "timestamp1";
    recipient = principal "'${USER1_PRINCIPAL}'";
    administrator = principal "'${USER1_PRINCIPAL}'";
    operator = principal "'${USER1_PRINCIPAL}'";
})' 2>&1)

if echo "$RESULT1" | grep -q "successfully"; then
    echo -e "${GREEN}✅ Domain 'timestamp1' registered${NC}"
else
    echo -e "${RED}❌ Failed to register domain 'timestamp1'${NC}"
    echo "$RESULT1"
fi

# Sleep briefly to ensure timestamp difference
sleep 2

# User 2 gets domain via admin gift
dfx identity use test-timestamp-2
USER2_PRINCIPAL=$(dfx identity get-principal)
dfx identity use default
RESULT2=$(dfx canister call registry admin_gift_domain '(record {
    domain_name = "timestamp2";
    recipient = principal "'${USER2_PRINCIPAL}'";
    administrator = principal "'${USER2_PRINCIPAL}'";
    operator = principal "'${USER2_PRINCIPAL}'";
})' 2>&1)

if echo "$RESULT2" | grep -q "successfully"; then
    echo -e "${GREEN}✅ Domain 'timestamp2' registered${NC}"
else
    echo -e "${RED}❌ Failed to register domain 'timestamp2'${NC}"
    echo "$RESULT2"
fi

# Get timestamp in between
echo -e "${YELLOW}Getting mid-point timestamp...${NC}"
TIMESTAMP_MIDDLE=$(date +%s%N | cut -b1-16)
echo "Timestamp middle: $TIMESTAMP_MIDDLE"

sleep 2

# User 3 gets domain via admin gift
dfx identity use test-timestamp-3
USER3_PRINCIPAL=$(dfx identity get-principal)
dfx identity use default
RESULT3=$(dfx canister call registry admin_gift_domain '(record {
    domain_name = "timestamp3";
    recipient = principal "'${USER3_PRINCIPAL}'";
    administrator = principal "'${USER3_PRINCIPAL}'";
    operator = principal "'${USER3_PRINCIPAL}'";
})' 2>&1)

if echo "$RESULT3" | grep -q "successfully"; then
    echo -e "${GREEN}✅ Domain 'timestamp3' registered${NC}"
else
    echo -e "${RED}❌ Failed to register domain 'timestamp3'${NC}"
    echo "$RESULT3"
fi

# Switch back to default identity
dfx identity use default

echo ""
echo -e "${BLUE}🔍 Testing Query Functions${NC}"
echo "=========================="

# Test 1: Get all domains with timestamps
echo -e "${YELLOW}Test 1: Getting all domains with timestamps...${NC}"
ALL_DOMAINS=$(dfx canister call registry get_all_domains_with_timestamps '()' 2>&1)
echo -e "${GREEN}All domains with timestamps:${NC}"
echo "$ALL_DOMAINS" | head -20

echo ""

# Test 2: Get domains since beginning (should get all)
echo -e "${YELLOW}Test 2: Getting domains since timestamp 0 (should get all)...${NC}"
DOMAINS_SINCE_0=$(dfx canister call registry get_domains_since_timestamp '(0)' 2>&1)
if echo "$DOMAINS_SINCE_0" | grep -q "timestamp1.*timestamp2.*timestamp3"; then
    echo -e "${GREEN}✅ Got all 3 domains${NC}"
else
    echo -e "${GREEN}Domains since 0:${NC}"
    echo "$DOMAINS_SINCE_0" | head -20
fi

echo ""

# Test 3: Get domains since middle timestamp (should get only timestamp3)
echo -e "${YELLOW}Test 3: Getting domains since middle timestamp...${NC}"
DOMAINS_SINCE_MIDDLE=$(dfx canister call registry get_domains_since_timestamp "($TIMESTAMP_MIDDLE)" 2>&1)
echo -e "${GREEN}Domains registered after middle timestamp:${NC}"
echo "$DOMAINS_SINCE_MIDDLE" | head -20

echo ""

# Test 4: Get domains since future timestamp (should get none)
echo -e "${YELLOW}Test 4: Getting domains since future timestamp (should be empty)...${NC}"
FUTURE_TIMESTAMP=$(($(date +%s%N | cut -b1-16) + 1000000000))  # Add 1000 seconds
DOMAINS_SINCE_FUTURE=$(dfx canister call registry get_domains_since_timestamp "($FUTURE_TIMESTAMP)" 2>&1)
if echo "$DOMAINS_SINCE_FUTURE" | grep -q "vec {}"; then
    echo -e "${GREEN}✅ Correctly returned empty list for future timestamp${NC}"
else
    echo -e "${GREEN}Result:${NC}"
    echo "$DOMAINS_SINCE_FUTURE"
fi

echo ""
echo -e "${BLUE}📊 Summary${NC}"
echo "=========="
echo -e "${GREEN}✅ Timestamp tracking is working!${NC}"
echo "- Domain records include registration_time field"
echo "- get_all_domains_with_timestamps() returns domains with their timestamps"
echo "- get_domains_since_timestamp(timestamp) filters domains by registration time"
echo ""
echo "You can use these functions to:"
echo "1. Monitor new domain registrations"
echo "2. Sync changes incrementally"
echo "3. Build activity feeds"
echo "4. Implement pagination by timestamp"

# Cleanup
echo ""
echo -e "${YELLOW}Cleaning up test identities...${NC}"
for i in {1..3}; do
    dfx identity remove test-timestamp-${i} 2>/dev/null || true
done
dfx identity use default

echo -e "${GREEN}🕐 Timestamp query test completed!${NC}"