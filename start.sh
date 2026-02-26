#!/bin/sh

set -e

# Start tailscaled daemon in the background
tailscaled --tun=userspace-networking --state="$TAILSCALE_STATE" --socket=/var/run/tailscale/tailscaled.sock &

# Wait for tailscaled socket to be available
sleep 3

# Authenticate and bring up tailscale
tailscale --socket=/var/run/tailscale/tailscaled.sock up \
  --authkey="${TAILSCALE_AUTHKEY}" \
  --hostname="${TAILSCALE_HOSTNAME:-tailscale-discloud}" \
  --accept-routes \
  --accept-dns

echo "[start.sh] Tailscale is up"

# Change to the working directory so the API binary resolves relative paths
# (./tunnels.json, ./public/) correctly.
cd /home/tailscale

# Start the tunnel manager API in the background
./api &

echo "[start.sh] API server started (working dir: $(pwd))"

# Wait for all background processes
wait
