#!/bin/bash
BIN=$(pwd)/target/release/git-ai

set -e

git config --global user.email "you@example.com"
git config --global user.name "Your Name"

rm -rf /tmp/git-ai
mkdir /tmp/git-ai
cd /tmp/git-ai
git init
echo "Hello World" > README.md
git add README.md
git commit -m "Initial commit"
echo "Hello World!" >> README.md
git add README.md
echo "git ai install"
$BIN install
echo "git commit"
git commit --no-edit
echo "git show"
git --no-pager show HEAD 
echo "git status"
git status -s
rm -rf /tmp/git-ai
