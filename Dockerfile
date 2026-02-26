FROM debian:bookworm-slim

ARG BUILD_SOURCE=remote
ARG TUNNEL_MANAGER_VERSION=latest
ARG GITHUB_REPO=jackskelt/tailscale-discloud

ENV DEBIAN_FRONTEND=noninteractive
ENV TAILSCALE_STATE=/home/discloud/tailscale.state
ENV TUNNELS_PATH=/home/discloud/tunnels.json

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    curl \
    iptables \
    iproute2 \
    socat \
    netcat-openbsd \
    jq \
    unzip \
    && rm -rf /var/lib/apt/lists/*

# Install Tailscale
RUN curl -fsSL https://tailscale.com/install.sh | sh

# Create tailscale user and directories
RUN groupadd -g 1000 tailscale && \
    useradd -u 1000 -g tailscale -m -d /home/tailscale -s /bin/bash tailscale

RUN mkdir -p /home/tailscale/public \
    /home/discloud \
    /var/lib/tailscale \
    /var/run/tailscale

# APP_FILES
RUN if [ "$BUILD_SOURCE" != "local" ]; then \
    if [ "$TUNNEL_MANAGER_VERSION" = "latest" ]; then \
    DOWNLOAD_URL=$(curl -fsSL "https://api.github.com/repos/${GITHUB_REPO}/releases/latest" \
    | jq -r '.assets[] | select(.name == "release.zip") | .browser_download_url'); \
    else \
    DOWNLOAD_URL="https://github.com/${GITHUB_REPO}/releases/download/${TUNNEL_MANAGER_VERSION}/release.zip"; \
    fi && \
    curl -fsSL -o /tmp/release.zip "$DOWNLOAD_URL" && \
    unzip -o /tmp/release.zip -d /home/tailscale && \
    rm /tmp/release.zip; \
    fi

RUN chmod +x /home/tailscale/api /home/tailscale/start.sh

# Fix permissions
RUN chown -R 1000:1000 /home/discloud /home/tailscale /var/lib/tailscale /var/run/tailscale

ENTRYPOINT ["/home/tailscale/start.sh"]
