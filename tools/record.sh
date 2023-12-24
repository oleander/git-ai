#!/bin/bash

# Directory for the test repository
DIR="/tmp/demo"
RESOURCES="$(pwd)/resources"

# Stop the script if any command fails
set -e

# Clean up any existing directory and create a new one
rm -rf $DIR
mkdir $DIR
cd $DIR

# Start asciinema recording
ASCIICAST_PATH="/tmp/demo.cast"
asciinema rec -i 1.5 -c "$0 record" $ASCIICAST_PATH &

# This function contains commands to be recorded
record() {
    # Initialize a new Git repository
    git init
    # Configure user details
    git config user.name "Test User"
    git config user.email "hello@world.com"
    git config --global init.defaultBranch main
    git branch -m main

    # Your Git AI commands
    git ai --help
    git ai examples
    git ai hook install
    echo "Hello World" > README.md
    git add README.md
    git commit --no-edit
    git show HEAD
}

# Check if the script is in "record" mode
if [ "$1" == "record" ]; then
    record
    exit
fi

# Wait for the recording process to finish
wait

# Convert the recording to a GIF
agg $ASCIICAST_PATH demo.gif
mv demo.gif $RESOURCES

# Clean up
cd ..
rm -rf $DIR
