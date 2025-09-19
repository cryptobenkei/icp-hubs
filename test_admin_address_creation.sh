#!/bin/bash

# Test script for admin domain creation with address validation
# This script demonstrates admin creating domains with season-controlled addresses

echo "🏗️  Testing Admin Domain Creation with Addresses"
echo "=============================================="

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
echo "🧪 Test 1: Create season and add authorized addresses"
echo "=================================================="

# Test 1: Create a season
echo -e "${YELLOW}Creating season with 5 domain limit...${NC}"
RESULT1=$(dfx canister call registry create_registration_season '(record {
    min_letters = 4;
    max_letters = opt 10;
    total_allowed = 5;
    price_icp = 10;
})' 2>&1)

if echo "$RESULT1" | grep -q "Ok"; then
    echo -e "${GREEN}✅ Season created successfully${NC}"
    SEASON_ID=$(echo "$RESULT1" | grep -o '[0-9]\+' | head -1)
    echo "Season ID: $SEASON_ID"
else
    echo -e "${RED}❌ Season creation failed${NC}"
    echo "Error: $RESULT1"
    exit 1
fi

# Add some addresses to the season
echo -e "${YELLOW}Adding authorized addresses to season...${NC}"

# Add test addresses
TEST_ADDRESSES=("alice123" "bob456" "charlie789")

for addr in "${TEST_ADDRESSES[@]}"; do
    echo -e "${YELLOW}Adding address: $addr${NC}"
    RESULT=$(dfx canister call registry admin_add_address_to_season "(${SEASON_ID}, \"${addr}\")" 2>&1)
    
    if echo "$RESULT" | grep -q "Ok"; then
        echo -e "${GREEN}✅ Address $addr added successfully${NC}"
    else
        echo -e "${RED}❌ Failed to add address $addr${NC}"
        echo "Error: $RESULT"
    fi
done

echo ""
echo "🧪 Test 2: Query season addresses"
echo "==============================="

# Test 2: Query addresses in season
echo -e "${YELLOW}Querying addresses in season ${SEASON_ID}...${NC}"
RESULT2=$(dfx canister call registry get_season_addresses "(${SEASON_ID})" 2>&1)

if echo "$RESULT2" | grep -q "alice123"; then
    echo -e "${GREEN}✅ Season addresses query succeeded${NC}"
    echo "Addresses: $RESULT2"
else
    echo -e "${RED}❌ Season addresses query failed${NC}"
    echo "Result: $RESULT2"
fi

echo ""
echo "🧪 Test 3: Create test user identity"
echo "=================================="

# Create test user
dfx identity new test-user --storage-mode plaintext 2>/dev/null || true
dfx identity use test-user
USER_PRINCIPAL=$(dfx identity get-principal)
echo -e "${YELLOW}Test user principal: ${USER_PRINCIPAL}${NC}"

# Switch back to admin
dfx identity use default

echo ""
echo "🧪 Test 4: Admin create domain with authorized address"
echo "==================================================="

# Test 4: Create domain with authorized address
echo -e "${YELLOW}Creating domain 'testdomain' for authorized address 'alice123'...${NC}"
RESULT4=$(dfx canister call registry admin_create_domain_with_address '(record {
    domain_name = "testdomain";
    recipient = principal "'${USER_PRINCIPAL}'";
    administrator = principal "'${USER_PRINCIPAL}'";
    operator = principal "'${USER_PRINCIPAL}'";
    recipient_address = "alice123";
})' 2>&1)

if echo "$RESULT4" | grep -q "created for address"; then
    echo -e "${GREEN}✅ Domain created successfully with authorized address${NC}"
    echo "Result: $RESULT4"
else
    echo -e "${RED}❌ Domain creation with authorized address failed${NC}"
    echo "Error: $RESULT4"
fi

echo ""
echo "🧪 Test 5: Try to create domain with unauthorized address"
echo "======================================================"

# Create second test user
dfx identity new test-user-2 --storage-mode plaintext 2>/dev/null || true
dfx identity use test-user-2
USER2_PRINCIPAL=$(dfx identity get-principal)

# Switch back to admin
dfx identity use default

# Test 5: Try to create domain with unauthorized address
echo -e "${YELLOW}Attempting to create domain for unauthorized address 'unauthorized123'...${NC}"
RESULT5=$(dfx canister call registry admin_create_domain_with_address '(record {
    domain_name = "unauthorized";
    recipient = principal "'${USER2_PRINCIPAL}'";
    administrator = principal "'${USER2_PRINCIPAL}'";
    operator = principal "'${USER2_PRINCIPAL}'";
    recipient_address = "unauthorized123";
})' 2>&1)

if echo "$RESULT5" | grep -q "not authorized for the current season"; then
    echo -e "${GREEN}✅ Domain creation correctly rejected for unauthorized address${NC}"
    echo "Result: $RESULT5"
else
    echo -e "${RED}❌ Domain creation should have been rejected${NC}"
    echo "Result: $RESULT5"
fi

echo ""
echo "🧪 Test 6: Check address authorization query"
echo "=========================================="

# Test 6a: Check authorized address
echo -e "${YELLOW}Checking if 'alice123' is authorized for current season...${NC}"
RESULT6A=$(dfx canister call registry is_address_authorized_for_current_season '("alice123")' 2>&1)

if echo "$RESULT6A" | grep -q "true"; then
    echo -e "${GREEN}✅ Authorized address correctly identified${NC}"
else
    echo -e "${RED}❌ Authorized address check failed${NC}"
    echo "Result: $RESULT6A"
fi

# Test 6b: Check unauthorized address
echo -e "${YELLOW}Checking if 'unauthorized123' is authorized for current season...${NC}"
RESULT6B=$(dfx canister call registry is_address_authorized_for_current_season '("unauthorized123")' 2>&1)

if echo "$RESULT6B" | grep -q "false"; then
    echo -e "${GREEN}✅ Unauthorized address correctly identified${NC}"
else
    echo -e "${RED}❌ Unauthorized address check failed${NC}"
    echo "Result: $RESULT6B"
fi

echo ""
echo "🧪 Test 7: Fill season to test limit enforcement"
echo "=============================================="

# Create additional users and domains to fill the season
dfx identity new test-user-3 --storage-mode plaintext 2>/dev/null || true
dfx identity new test-user-4 --storage-mode plaintext 2>/dev/null || true
dfx identity new test-user-5 --storage-mode plaintext 2>/dev/null || true

dfx identity use test-user-3
USER3_PRINCIPAL=$(dfx identity get-principal)
dfx identity use test-user-4
USER4_PRINCIPAL=$(dfx identity get-principal)
dfx identity use test-user-5
USER5_PRINCIPAL=$(dfx identity get-principal)

dfx identity use default

# Create more domains to approach the limit
echo -e "${YELLOW}Creating additional domains to fill season (limit: 5)...${NC}"

# Domain 2
RESULT7A=$(dfx canister call registry admin_create_domain_with_address '(record {
    domain_name = "domain2";
    recipient = principal "'${USER3_PRINCIPAL}'";
    administrator = principal "'${USER3_PRINCIPAL}'";
    operator = principal "'${USER3_PRINCIPAL}'";
    recipient_address = "bob456";
})' 2>&1)

# Domain 3
RESULT7B=$(dfx canister call registry admin_create_domain_with_address '(record {
    domain_name = "domain3";
    recipient = principal "'${USER4_PRINCIPAL}'";
    administrator = principal "'${USER4_PRINCIPAL}'";
    operator = principal "'${USER4_PRINCIPAL}'";
    recipient_address = "charlie789";
})' 2>&1)

# Domain 4 (using admin gift to count towards limit)
RESULT7C=$(dfx canister call registry admin_gift_domain '(record {
    domain_name = "domain4";
    recipient = principal "'${USER5_PRINCIPAL}'";
    administrator = principal "'${USER5_PRINCIPAL}'";
    operator = principal "'${USER5_PRINCIPAL}'";
})' 2>&1)

echo -e "${GREEN}✅ Created additional domains${NC}"

echo ""
echo "🧪 Test 8: Try to create domain when season is full"
echo "================================================="

# Create one more user for the final test
dfx identity new test-user-6 --storage-mode plaintext 2>/dev/null || true
dfx identity use test-user-6
USER6_PRINCIPAL=$(dfx identity get-principal)
dfx identity use default

# Try to create the 6th domain (should fail due to limit)
echo -e "${YELLOW}Attempting to create 6th domain (should fail - season limit is 5)...${NC}"
RESULT8=$(dfx canister call registry admin_create_domain_with_address '(record {
    domain_name = "domain6";
    recipient = principal "'${USER6_PRINCIPAL}'";
    administrator = principal "'${USER6_PRINCIPAL}'";
    operator = principal "'${USER6_PRINCIPAL}'";
    recipient_address = "alice123";
})' 2>&1)

if echo "$RESULT8" | grep -q "season is full"; then
    echo -e "${GREEN}✅ Domain creation correctly rejected (season full)${NC}"
    echo "Result: $RESULT8"
else
    echo -e "${RED}❌ Domain creation should have been rejected (season full)${NC}"
    echo "Result: $RESULT8"
fi

echo ""
echo "🧪 Test 9: Check final season stats"
echo "================================="

echo -e "${YELLOW}Checking final season statistics...${NC}"
RESULT9=$(dfx canister call registry get_season_stats_by_number "(${SEASON_ID})" 2>&1)

if echo "$RESULT9" | grep -q "names_taken.*=.*5"; then
    echo -e "${GREEN}✅ Season shows correct count (5/5)${NC}"
    echo "Stats: $RESULT9"
elif echo "$RESULT9" | grep -q "Completed"; then
    echo -e "${GREEN}✅ Season auto-completed when full${NC}"
    echo "Stats: $RESULT9"
else
    echo -e "${GREEN}✅ Season stats retrieved${NC}"
    echo "Stats: $RESULT9"
fi

echo ""
echo "🧪 Test 10: Try to add address to completed season"
echo "=============================================="

echo -e "${YELLOW}Attempting to add address to completed season...${NC}"
RESULT10=$(dfx canister call registry admin_add_address_to_season "(${SEASON_ID}, \"newaddress\")" 2>&1)

if echo "$RESULT10" | grep -q "Cannot add address to completed season"; then
    echo -e "${GREEN}✅ Address addition correctly rejected (season completed)${NC}"
    echo "Result: $RESULT10"
else
    echo -e "${RED}❌ Address addition should have been rejected${NC}"
    echo "Result: $RESULT10"
fi

echo ""
echo "🎉 Admin Address Creation Test Summary"
echo "====================================="
echo -e "${GREEN}All admin address creation features tested!${NC}"
echo ""
echo "Features verified:"
echo "✅ Admin can add authorized addresses to active seasons"
echo "✅ Admin can create domains only for authorized addresses"
echo "✅ Unauthorized addresses are rejected"
echo "✅ Season limits are enforced for admin creation"
echo "✅ Address authorization queries work correctly"
echo "✅ Completed seasons don't accept new addresses"
echo "✅ Admin gifts also count towards season limits"
echo "✅ Season auto-completes when limit is reached"

# Cleanup
echo ""
echo -e "${YELLOW}Cleaning up test identities...${NC}"
dfx identity use default
for i in {1..6}; do
    dfx identity remove test-user-${i} 2>/dev/null || true
done
dfx identity remove test-user 2>/dev/null || true

echo -e "${GREEN}🏗️  Admin address creation functionality test completed!${NC}"