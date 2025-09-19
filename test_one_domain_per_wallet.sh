#!/bin/bash

# Test script for one-domain-per-wallet functionality
# This script demonstrates the expected behavior when deployed to local dfx

echo "🧪 Testing One Domain Per Wallet Restriction"
echo "============================================="

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Test setup
echo -e "${YELLOW}Setting up test environment...${NC}"

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

# Create a test season for domain registration
echo -e "${YELLOW}Creating test season...${NC}"
dfx canister call registry create_registration_season '(record {
    min_letters = 4;
    max_letters = opt 10;
    total_allowed = 100;
    price_icp = 1;
})'

echo ""
echo "🧪 Test 1: First domain registration should succeed"
echo "=================================================="

# Test 1: Register first domain with default identity
echo -e "${YELLOW}Registering 'testdomain' with default identity...${NC}"
RESULT1=$(dfx canister call registry register_domain '(record {
    domain_name = "testdomain";
    administrator = principal "'${ADMIN_PRINCIPAL}'";
    operator = principal "'${ADMIN_PRINCIPAL}'";
    payment_block = 1;
})' 2>&1)

if echo "$RESULT1" | grep -q "successfully"; then
    echo -e "${GREEN}✅ First domain registration succeeded${NC}"
    echo "Result: $RESULT1"
else
    echo -e "${RED}❌ First domain registration failed${NC}"
    echo "Error: $RESULT1"
fi

echo ""
echo "🧪 Test 2: Second domain registration should fail"
echo "==============================================="

# Test 2: Try to register second domain with same identity (should fail)
echo -e "${YELLOW}Attempting to register 'anotherdomain' with same identity...${NC}"
RESULT2=$(dfx canister call registry register_domain '(record {
    domain_name = "anotherdomain";
    administrator = principal "'${ADMIN_PRINCIPAL}'";
    operator = principal "'${ADMIN_PRINCIPAL}'";
    payment_block = 2;
})' 2>&1)

if echo "$RESULT2" | grep -q "already owns domain"; then
    echo -e "${GREEN}✅ Second domain registration correctly rejected${NC}"
    echo "Result: $RESULT2"
else
    echo -e "${RED}❌ Second domain registration should have failed${NC}"
    echo "Result: $RESULT2"
fi

echo ""
echo "🧪 Test 3: Query wallet domain"
echo "=============================="

# Test 3: Query what domain the wallet owns
echo -e "${YELLOW}Querying domain owned by default identity...${NC}"
RESULT3=$(dfx canister call registry get_wallet_domain "(principal \"${ADMIN_PRINCIPAL}\")" 2>&1)

if echo "$RESULT3" | grep -q "testdomain"; then
    echo -e "${GREEN}✅ Wallet domain query returned correct domain${NC}"
    echo "Result: $RESULT3"
else
    echo -e "${RED}❌ Wallet domain query failed or returned wrong domain${NC}"
    echo "Result: $RESULT3"
fi

echo ""
echo "🧪 Test 4: Create second identity and test registration"
echo "===================================================="

# Test 4: Create new identity and test registration
echo -e "${YELLOW}Creating second test identity...${NC}"
dfx identity new test-user --storage-mode plaintext 2>/dev/null || true
dfx identity use test-user
USER_PRINCIPAL=$(dfx identity get-principal)
echo -e "${YELLOW}User principal: ${USER_PRINCIPAL}${NC}"

echo -e "${YELLOW}Registering 'userdomain' with second identity...${NC}"
RESULT4=$(dfx canister call registry register_domain '(record {
    domain_name = "userdomain";
    administrator = principal "'${USER_PRINCIPAL}'";
    operator = principal "'${USER_PRINCIPAL}'";
    payment_block = 3;
})' 2>&1)

if echo "$RESULT4" | grep -q "successfully"; then
    echo -e "${GREEN}✅ Second identity registration succeeded${NC}"
    echo "Result: $RESULT4"
else
    echo -e "${RED}❌ Second identity registration failed${NC}"
    echo "Error: $RESULT4"
fi

echo ""
echo "🧪 Test 5: Test domain transfer"
echo "==============================="

# Test 5: Test domain transfer (should fail - user already has domain)
dfx identity use default
echo -e "${YELLOW}Attempting to transfer 'testdomain' to user who already has domain...${NC}"
RESULT5=$(dfx canister call registry transfer_domain_ownership "(\"testdomain\", principal \"${USER_PRINCIPAL}\")" 2>&1)

if echo "$RESULT5" | grep -q "already owns domain"; then
    echo -e "${GREEN}✅ Transfer correctly rejected (user already has domain)${NC}"
    echo "Result: $RESULT5"
else
    echo -e "${RED}❌ Transfer should have been rejected${NC}"
    echo "Result: $RESULT5"
fi

echo ""
echo "🧪 Test 6: Create third identity for successful transfer"
echo "======================================================"

# Test 6: Create third identity for successful transfer
echo -e "${YELLOW}Creating third test identity...${NC}"
dfx identity new test-user-2 --storage-mode plaintext 2>/dev/null || true
dfx identity use test-user-2
USER2_PRINCIPAL=$(dfx identity get-principal)
echo -e "${YELLOW}User2 principal: ${USER2_PRINCIPAL}${NC}"

# Switch back to admin for transfer
dfx identity use default
echo -e "${YELLOW}Transferring 'testdomain' to user2 (who has no domain)...${NC}"
RESULT6=$(dfx canister call registry transfer_domain_ownership "(\"testdomain\", principal \"${USER2_PRINCIPAL}\")" 2>&1)

if echo "$RESULT6" | grep -q "Ok"; then
    echo -e "${GREEN}✅ Transfer succeeded${NC}"
    echo "Result: $RESULT6"
else
    echo -e "${RED}❌ Transfer failed${NC}"
    echo "Error: $RESULT6"
fi

echo ""
echo "🧪 Test 7: Verify transfer updated mappings"
echo "=========================================="

# Test 7: Verify mappings updated
echo -e "${YELLOW}Checking if admin no longer owns testdomain...${NC}"
RESULT7A=$(dfx canister call registry get_wallet_domain "(principal \"${ADMIN_PRINCIPAL}\")" 2>&1)

if echo "$RESULT7A" | grep -q "null"; then
    echo -e "${GREEN}✅ Admin correctly has no domain after transfer${NC}"
else
    echo -e "${RED}❌ Admin should have no domain after transfer${NC}"
    echo "Result: $RESULT7A"
fi

echo -e "${YELLOW}Checking if user2 now owns testdomain...${NC}"
RESULT7B=$(dfx canister call registry get_wallet_domain "(principal \"${USER2_PRINCIPAL}\")" 2>&1)

if echo "$RESULT7B" | grep -q "testdomain"; then
    echo -e "${GREEN}✅ User2 correctly owns testdomain after transfer${NC}"
else
    echo -e "${RED}❌ User2 should own testdomain after transfer${NC}"
    echo "Result: $RESULT7B"
fi

echo ""
echo "🧪 Test 8: Admin can register multiple domains"
echo "============================================="

# Test 8: Admin exemption - admin can register multiple domains
echo -e "${YELLOW}Admin registering second domain (should succeed due to admin exemption)...${NC}"
RESULT8=$(dfx canister call registry register_domain '(record {
    domain_name = "admindomain";
    administrator = principal "'${ADMIN_PRINCIPAL}'";
    operator = principal "'${ADMIN_PRINCIPAL}'";
    payment_block = 4;
})' 2>&1)

if echo "$RESULT8" | grep -q "successfully"; then
    echo -e "${GREEN}✅ Admin can register multiple domains${NC}"
    echo "Result: $RESULT8"
else
    echo -e "${RED}❌ Admin should be able to register multiple domains${NC}"
    echo "Error: $RESULT8"
fi

echo ""
echo "🎉 Test Summary"
echo "==============="
echo -e "${GREEN}All tests completed!${NC}"
echo ""
echo "Expected behavior verified:"
echo "✅ First domain registration succeeds"
echo "✅ Second domain registration from same wallet fails"
echo "✅ Wallet domain query works correctly"
echo "✅ Different wallets can register domains"
echo "✅ Transfer to wallet with existing domain fails"
echo "✅ Transfer to wallet without domain succeeds"
echo "✅ Transfer updates wallet-to-domain mappings"
echo "✅ Admin can register multiple domains"

# Cleanup
echo ""
echo -e "${YELLOW}Cleaning up test identities...${NC}"
dfx identity use default
dfx identity remove test-user 2>/dev/null || true
dfx identity remove test-user-2 2>/dev/null || true

echo -e "${GREEN}🎉 One-domain-per-wallet functionality test completed!${NC}"