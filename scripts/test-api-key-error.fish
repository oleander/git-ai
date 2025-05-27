#!/usr/bin/env fish

# Test script to verify API key error handling

set -x RUST_LOG error

function test_step
    set description $argv[1]
    echo "=== Testing: $description ==="
end

function fail
    echo "❌ Test failed: $argv"
    exit 1
end

function success
    echo "✅ Test passed: $argv"
end

# Create a temporary directory for testing
set TEST_DIR (mktemp -d)
cd $TEST_DIR

# Initialize a git repository
git init || fail "Git init failed"
git config user.name "Test User"
git config user.email "test@example.com"

# Install the hook
git-ai hook install || fail "Hook installation failed"

# Create a test file
echo "Hello World" > test.txt
git add test.txt

test_step "Invalid API key should fail with clear error"

# Set an invalid API key
set -x OPENAI_API_KEY "invalid_key_12345"

# Try to commit with git-ai (this should fail with a clear error)
if git commit --no-edit 2>&1 | grep -q "Invalid OpenAI API key"
    success "Invalid API key error was properly caught"
else
    fail "Invalid API key error was not properly handled"
end

test_step "Missing API key should fail with clear error"

# Unset the API key
set -e OPENAI_API_KEY

# Try to commit with git-ai (this should fail with a clear error)
if git commit --no-edit 2>&1 | grep -q "OpenAI API key not configured"
    success "Missing API key error was properly caught"
else
    fail "Missing API key error was not properly handled"
end

# Clean up
cd ..
rm -rf $TEST_DIR

echo ""
echo "�� All tests passed!"
