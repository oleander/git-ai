#!/bin/bash

# Check if OPENAI_API_KEY is defined
if [ -z "${OPENAI_API_KEY}" ]; then
  echo "Error: OPENAI_API_KEY is not defined."
  exit 1
fi

# Test Variables
OLLAMA_MODEL="smollm:135m"
OLLAMA_HOST="http://localhost:11434"
INVALID_OLLAMA_HOST="invalid_host"
INVALID_OLLAMA_MODEL="nonexistent-model"
OPENAI_INVALID_KEY="invalid_key"
OPENAI_VALID_KEY="${OPENAI_API_KEY}"
OPENAI_MODEL="gpt-4o-mini"

set -e          # Exit immediately if a command exits with a non-zero status
set -o pipefail # Catch errors in pipes
set -x          # Print each command before executing

# Create a test Git repository
mkdir -p /test-repo
cd /test-repo
git init

# Configure Git user
git config --global user.name "Test User"
git config --global user.email "test@example.com"

# Test: git-ai should be installed
git-ai --version

# Hook Installation Tests
git ai hook install
git ai hook install && echo "Expected failure but passed" && exit 1 || echo "Expected failure"

git ai hook reinstall
git ai hook reinstall # Should pass

git ai hook uninstall && echo "Expected failure but passed" && exit 1 || echo "Expected failure"
git ai hook uninstall
git ai hook install

# Test: Config provider to Ollama
git ai config provider ollama
git ai config ollama --model ${OLLAMA_MODEL}

# Test: Committing with git-ai (should fail initially)
echo "test" >file.txt
git add file.txt
git commit --no-edit && echo "Expected failure but passed" && exit 1 || echo "Expected failure"

# Test: Setting valid provider and retrying commit
git ai config provider ollama
git ai config ollama --model ${OLLAMA_MODEL}
git commit --no-edit # Should succeed

# Test: Invalid host configuration
git ai config ollama --host "${INVALID_OLLAMA_HOST}"
git add file2.txt
echo "another test" >file2.txt
git commit --no-edit && echo "Expected failure but passed" && exit 1 || echo "Expected failure"

# Reset host and retry
git ai config ollama --host "${OLLAMA_HOST}"
git commit --no-edit # Should succeed

# Test: Invalid model
git ai config ollama --model "${INVALID_OLLAMA_MODEL}" && echo "Expected failure but passed" && exit 1 || echo "Expected failure"

# Reset model
git ai config ollama --model ${OLLAMA_MODEL}

# OpenAI Tests
git ai config provider openai
git ai config openai --api-key "${OPENAI_INVALID_KEY}" && echo "Expected failure but passed" && exit 1 || echo "Expected failure"

git ai config openai --api-key "${OPENAI_VALID_KEY}"
git ai config openai --model ${OPENAI_MODEL}
git commit --no-edit # Should succeed

# Test: Config reset
git ai config reset
git ai config provider # Should require reconfiguration

echo "All tests passed successfully!"
exit 0
