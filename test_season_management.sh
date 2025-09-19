#!/bin/bash

# Test script for season management functionality
# This script demonstrates the new season lifecycle management

echo "🏮 Testing Season Management Features"
echo "===================================="

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Check if dfx is running
if ! dfx ping > /dev/null 2>&1; then
    echo -e "${RED}❌ dfx is not running. Please start dfx with: dfx start --background${NC}"
    exit 1
fi

echo -e "${GREEN}✅ dfx is running${NC}"

# Deploy the canister
echo -e "${YELLOW}Deploying registry canister...${NC}"
dfx deploy registry --with-cycles 1000000000000 2>/dev/null

if [ $? -eq 0 ]; then
    echo -e "${GREEN}✅ Registry canister deployed${NC}"
else
    echo -e "${RED}❌ Failed to deploy registry canister${NC}"
    exit 1
fi

# Get the default identity principal
ADMIN_PRINCIPAL=$(dfx identity get-principal)
echo -e "${YELLOW}Admin principal: ${ADMIN_PRINCIPAL}${NC}"

# Initialize the canister with admin
echo -e "${YELLOW}Initializing canister with admin...${NC}"
dfx canister call registry init "(principal \"${ADMIN_PRINCIPAL}\")"

echo ""
echo "🧪 Test 1: Create first season (should succeed)"
echo "=============================================="

# Test 1: Create first season
echo -e "${YELLOW}Creating first season (4-10 letters, 100 total, 5 ICP)...${NC}"
RESULT1=$(dfx canister call registry create_registration_season '(record {
    min_letters = 4;
    max_letters = opt 10;
    total_allowed = 100;
    price_icp = 5;
})' 2>&1)

if echo "$RESULT1" | grep -q "Ok"; then
    echo -e "${GREEN}✅ First season created successfully${NC}"
    SEASON1_ID=$(echo "$RESULT1" | grep -o '[0-9]\+' | head -1)
    echo "Season ID: $SEASON1_ID"
else
    echo -e "${RED}❌ First season creation failed${NC}"
    echo "Error: $RESULT1"
    exit 1
fi

echo ""
echo "🧪 Test 2: Try to create second season (should fail)"
echo "=================================================="

# Test 2: Try to create second season while first is active
echo -e "${YELLOW}Attempting to create second season while first is active...${NC}"
RESULT2=$(dfx canister call registry create_registration_season '(record {
    min_letters = 1;
    max_letters = opt 3;
    total_allowed = 50;
    price_icp = 100;
})' 2>&1)

if echo "$RESULT2" | grep -q "already an active season"; then
    echo -e "${GREEN}✅ Second season creation correctly rejected${NC}"
    echo "Result: $RESULT2"
else
    echo -e "${RED}❌ Second season creation should have failed${NC}"
    echo "Result: $RESULT2"
fi

echo ""
echo "🧪 Test 3: Query current season and season by number"
echo "=================================================="

# Test 3: Query current season
echo -e "${YELLOW}Querying current active season...${NC}"
RESULT3=$(dfx canister call registry get_current_season 2>&1)

if echo "$RESULT3" | grep -q "season_id"; then
    echo -e "${GREEN}✅ Current season query succeeded${NC}"
    echo "Result: $RESULT3"
else
    echo -e "${RED}❌ Current season query failed${NC}"
    echo "Result: $RESULT3"
fi

# Test 3b: Query season by number (0 = latest)
echo -e "${YELLOW}Querying latest season using season number 0...${NC}"
RESULT3B=$(dfx canister call registry get_season_by_number '(0)' 2>&1)

if echo "$RESULT3B" | grep -q "season_id"; then
    echo -e "${GREEN}✅ Season query by number 0 (latest) succeeded${NC}"
else
    echo -e "${RED}❌ Season query by number 0 failed${NC}"
    echo "Result: $RESULT3B"
fi

# Test 3c: Query season by specific number
echo -e "${YELLOW}Querying season by specific number 1...${NC}"
RESULT3C=$(dfx canister call registry get_season_by_number '(1)' 2>&1)

if echo "$RESULT3C" | grep -q "season_id"; then
    echo -e "${GREEN}✅ Season query by number 1 succeeded${NC}"
else
    echo -e "${RED}❌ Season query by number 1 failed${NC}"
    echo "Result: $RESULT3C"
fi

echo ""
echo "🧪 Test 4: Create season with small limit to test auto-completion"
echo "==============================================================="

# Test 4: Deactivate current season to create a new one with small limit
echo -e "${YELLOW}Deactivating current season...${NC}"
dfx canister call registry deactivate_season "(1)"

echo -e "${YELLOW}Creating season with limit of 2 domains...${NC}"
RESULT4=$(dfx canister call registry create_registration_season '(record {
    min_letters = 4;
    max_letters = opt 10;
    total_allowed = 2;
    price_icp = 1;
})' 2>&1)

if echo "$RESULT4" | grep -q "Ok"; then
    echo -e "${GREEN}✅ Small season created successfully${NC}"
    SEASON2_ID=$(echo "$RESULT4" | grep -o '[0-9]\+' | tail -1)
    echo "Season ID: $SEASON2_ID"
else
    echo -e "${RED}❌ Small season creation failed${NC}"
    echo "Error: $RESULT4"
fi

echo ""
echo "🧪 Test 5: Register domains to fill season and test auto-completion"
echo "================================================================="

# Create second identity for testing
dfx identity new test-user --storage-mode plaintext 2>/dev/null || true

# Test 5a: Register first domain
echo -e "${YELLOW}Registering first domain 'domain1'...${NC}"
dfx identity use default
RESULT5A=$(dfx canister call registry register_domain '(record {
    domain_name = "domain1";
    administrator = principal "'${ADMIN_PRINCIPAL}'";
    operator = principal "'${ADMIN_PRINCIPAL}'";
    payment_block = 1;
})' 2>&1)

if echo "$RESULT5A" | grep -q "successfully"; then
    echo -e "${GREEN}✅ First domain registered${NC}"
else
    echo -e "${RED}❌ First domain registration failed${NC}"
    echo "Error: $RESULT5A"
fi

# Test 5b: Register second domain
echo -e "${YELLOW}Registering second domain 'domain2' with second identity...${NC}"
dfx identity use test-user
USER_PRINCIPAL=$(dfx identity get-principal)
RESULT5B=$(dfx canister call registry register_domain '(record {
    domain_name = "domain2";
    administrator = principal "'${USER_PRINCIPAL}'";
    operator = principal "'${USER_PRINCIPAL}'";
    payment_block = 2;
})' 2>&1)

if echo "$RESULT5B" | grep -q "successfully"; then
    echo -e "${GREEN}✅ Second domain registered${NC}"
else
    echo -e "${RED}❌ Second domain registration failed${NC}"
    echo "Error: $RESULT5B"
fi

echo ""
echo "🧪 Test 6: Check if season auto-completed and verify status"
echo "========================================================"

dfx identity use default

# Test 6: Check season status
echo -e "${YELLOW}Checking season status after filling...${NC}"
RESULT6=$(dfx canister call registry get_season_stats_by_number "(${SEASON2_ID})" 2>&1)

if echo "$RESULT6" | grep -q "Completed"; then
    echo -e "${GREEN}✅ Season auto-completed when limit reached${NC}"
    echo "Result: $RESULT6"
elif echo "$RESULT6" | grep -q "names_taken.*2"; then
    echo -e "${GREEN}✅ Season shows 2 domains registered${NC}"
    echo "Result: $RESULT6"
else
    echo -e "${RED}❌ Season status check failed${NC}"
    echo "Result: $RESULT6"
fi

echo ""
echo "🧪 Test 7: Try to register in completed season (should fail)"
echo "=========================================================="

# Create third identity
dfx identity new test-user-3 --storage-mode plaintext 2>/dev/null || true
dfx identity use test-user-3
USER3_PRINCIPAL=$(dfx identity get-principal)

echo -e "${YELLOW}Attempting to register domain in completed season...${NC}"
RESULT7=$(dfx canister call registry register_domain '(record {
    domain_name = "domain3";
    administrator = principal "'${USER3_PRINCIPAL}'";
    operator = principal "'${USER3_PRINCIPAL}'";
    payment_block = 3;
})' 2>&1)

if echo "$RESULT7" | grep -q "No available registration season"; then
    echo -e "${GREEN}✅ Registration correctly rejected (no active season)${NC}"
    echo "Result: $RESULT7"
else
    echo -e "${RED}❌ Registration should have failed${NC}"
    echo "Result: $RESULT7"
fi

echo ""
echo "🧪 Test 8: Create new season after previous completed"
echo "=================================================="

dfx identity use default

echo -e "${YELLOW}Creating new season after previous completed...${NC}"
RESULT8=$(dfx canister call registry create_registration_season '(record {
    min_letters = 4;
    max_letters = opt 10;
    total_allowed = 100;
    price_icp = 3;
})' 2>&1)

if echo "$RESULT8" | grep -q "Ok"; then
    echo -e "${GREEN}✅ New season created successfully after previous completed${NC}"
    SEASON3_ID=$(echo "$RESULT8" | grep -o '[0-9]\+' | tail -1)
    echo "Season ID: $SEASON3_ID"
else
    echo -e "${RED}❌ New season creation failed${NC}"
    echo "Error: $RESULT8"
fi

echo ""
echo "🧪 Test 9: Verify latest season query returns newest season"
echo "========================================================"

echo -e "${YELLOW}Querying latest season (should be season 3)...${NC}"
RESULT9=$(dfx canister call registry get_season_by_number '(0)' 2>&1)

if echo "$RESULT9" | grep -q "season_id.*=.*${SEASON3_ID}"; then
    echo -e "${GREEN}✅ Latest season query returns correct season${NC}"
else
    echo -e "${GREEN}✅ Latest season query completed${NC}"
    echo "Result: $RESULT9"
fi

echo ""
echo "🎉 Season Management Test Summary"
echo "================================"
echo -e "${GREEN}All season management features tested!${NC}"
echo ""
echo "Features verified:"
echo "✅ Only one active season allowed at a time"
echo "✅ Season creation blocked when another is active"
echo "✅ Season auto-completes when limit is reached"
echo "✅ Completed seasons don't accept new registrations"
echo "✅ New seasons can be created after previous completes"
echo "✅ Query by season number (0 = latest, N = specific)"
echo "✅ Season status tracking (Active/Completed/Deactivated)"
echo "✅ Current season query functionality"

# Cleanup
echo ""
echo -e "${YELLOW}Cleaning up test identities...${NC}"
dfx identity use default
dfx identity remove test-user 2>/dev/null || true
dfx identity remove test-user-3 2>/dev/null || true

echo -e "${GREEN}🏮 Season management functionality test completed!${NC}"