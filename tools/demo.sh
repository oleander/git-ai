#!/bin/bash

export ZSH_AUTOSUGGEST_HISTORY_IGNORE=*

CAST_PATH="/tmp/demo.cast"
rm -f $CAST_PATH

DEMO_PATH="/tmp/demo"
rm -rf $DEMO_PATH

mkdir $DEMO_PATH
cd $DEMO_PATH
git init
echo "Hello World" > README.md
git add README.md
git commit -m "Initial commit"

asciinema rec -i 1.5 $CAST_PATH

# git ai hook install
# echo "Hello World" > README.md
# git add README.md
# git commit --no-edit
