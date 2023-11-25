#!env bash

set -e

echo "Starting watch script"
gh run watch && osascript -e 'display notification "GitHub Action Completed" with title "Workflow Status"'
sleep 10
./watch.sh