#!/bin/bash

# List all files not ignored by .gitignore
files=$(git ls-files)

# Iterate over the files and display their contents
for file in $files; do
    echo "Contents of $file:"
    cat "$file"
    echo "--------------------------------"
done