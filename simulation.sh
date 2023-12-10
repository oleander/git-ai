#!/bin/bash
BIN=$(pwd)/target/release/git-ai

set -e

rm -rf /tmp/git-ai
mkdir /tmp/git-ai
cd /tmp/git-ai
git init
echo "Hello World" > README.md
git add README.md
git commit -m "Initial commit"
echo "Hello World!" >> README.md
git add README.md
$BIN install
git commit --no-edit
git --no-pager show HEAD 
git status | grep -q 'nothing to commit' || echo "Commit failed" && exit 1
rm -rf /tmp/git-ai
