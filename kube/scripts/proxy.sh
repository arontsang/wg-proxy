#!/usr/bin/env sh
echo "Listening..."
SCRIPT_DIR=$(dirname "$0")
socat TCP-LISTEN:3128,reuseaddr,fork EXEC:$SCRIPT_DIR/forward.sh