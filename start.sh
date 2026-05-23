#!/bin/sh

set -e

# In production, the API binary handles starting and waiting for tailscaled
cd /home/tailscale
exec ./api --prod
