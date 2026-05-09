#!/usr/bin/env sh
curl -s --max-time 10 -o /dev/null $PREWARM_ENDPOINT
while true; do
    curl -s -o /dev/null $PING_ENDPOINT
    sleep 5
done &
PING_PID=\$!
# # Trap to kill the loop on exit
trap 'kill \$PING_PID 2>/dev/null' EXIT

nc $HTTP_TUNNEL_ADDRESS $HTTP_TUNNEL_PORT