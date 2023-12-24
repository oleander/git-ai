#!/bin/bash

DIR="/tmp/git-ai-test"

set -e
rm -rf $DIR
mkdir $DIR
cd $DIR

git init
git config user.name "Test User"
git config user.email hello@world.com

git ai hook install
git ai config set openai-api-key $OPENAI_API_KEY

# Create a commit
echo "Hello World" > README.md
git add README.md
git commit -m "Initial commit"

git ai examples

echo "Hello World 2" > README.md
git add README.md
git commit --no-edit

git status
cd ..
rm -rf $DIR
