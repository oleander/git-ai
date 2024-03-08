#!/bin/bash

if [ ! -f .env.local ]; then
  echo ".env.local missing"
  exit 1
fi

if [ ! -f .env ]; then
  echo ".env missing"
  exit 1
fi

source .env
source .env.local

# Directory for the test repository
DIR="/tmp/git-ai-test"

# Stop the script if any command fails set -e

# Install the git-ai
cargo install --debug --path .

# Clean up any existing directory and create a new one
rm -rf $DIR
mkdir $DIR
cd $DIR

if [ -z "$OPENAI_API_KEY" ]; then
  echo "Please set the OPENAI_API_KEY environment variable."
  exit 1
fi

# Initialize a new Git repository
git init
# Configure user details
git config user.name "Test User"
git config user.email "hello@world.com"
git config --global init.defaultBranch main
git branch -m main

# Test the git-ai hook installation and uninstallation
echo "Testing git-ai hook installation..."
git-ai hook install
echo "Testing git-ai hook uninstallation..."
git-ai hook uninstall
echo "Re-testing git-ai hook installation..."
git-ai hook install

# Set various configuration values
echo "Setting configuration values..."
git-ai config set model gpt-4
git-ai config set language en
git-ai config set max-diff-tokens 1500
git-ai config set max-length 72
git-ai config set openai-api-key $OPENAI_API_KEY

# Create a commit to test hook functionality
echo "Hello World" > README.md
git add README.md
git commit -m "Initial commit"

# Run git-ai examples
echo "Running git-ai examples..."
git-ai examples

# Modify the file and create another commit to test hook functionality
echo "Hello World 2" > README.md
git add README.md
git commit --no-edit

git commit --amend --no-edit

# Check the status of the repository
git status

# Cleanup: go back to the original directory and remove the test repository
cd ..
rm -rf $DIR

echo "All tests completed successfully."
