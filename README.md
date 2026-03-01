<div align="center">
  <img src="public/icon.webp" alt="Tailscale" width="120" />
  <h1>Tailscale Tunnel Manager</h1>
  <p>A self-hosted TCP tunnel manager for Tailscale containers, designed to run on <a href="https://discloud.com">Discloud</a>.</p>
  <img src="images/banner.png" alt="Banner" />
</div>

---

## About

Tailscale Tunnel Manager is a lightweight application that lets you create, manage, and monitor TCP tunnels inside a Tailscale-connected container. It exposes local ports on a Tailscale node and forwards traffic to arbitrary hosts and ports using [socat](https://linux.die.net/man/1/socat), all managed through a web interface and a REST API.

The primary use case is running the manager on a Discloud container so that services deployed alongside it (databases, internal tools) become accessible over your Tailscale network without exposing them to the public internet.

## Features

- **Web dashboard** -- Create, edit, toggle, and delete tunnels from the browser. Includes dark and light themes.
- **Quick start templates** -- Pre-configured templates for common services such as PostgreSQL, MySQL, Redis, and MongoDB.
- **Connection testing** -- Test target reachability directly from the UI before or after creating a tunnel.
- **Tunnel persistence** -- Tunnel configuration is saved to disk and automatically restored on container restart. Tunnels that fail to restore are disabled instead of retrying indefinitely.
- **Internationalization** -- The interface is available in English, Portuguese (BR), Spanish, French, German, and Japanese.
- **Statically linked binary** -- The API server is compiled for `x86_64-unknown-linux-musl`, producing a fully static binary with no runtime dependencies.

## REST API

The API server listens on port `3000` and serves both the static frontend and the following endpoints:

| Method | Endpoint | Description |
| ------ | ---------------- | ---------------------------------------- |
| GET | `/api/config` | Returns the current Tailscale hostname. |
| GET | `/api/tunnels` | Lists all tunnels with connection URLs. |
| POST | `/api/tunnels` | Creates a new tunnel. |
| PUT | `/api/tunnels/:id`| Updates an existing tunnel. |
| DELETE | `/api/tunnels/:id`| Stops and deletes a tunnel. |
| POST | `/api/test` | Tests TCP connectivity to a host:port. |

## Environment Variables

| Variable | Default | Description |
| -------------------- | ----------------------- | -------------------------------------------------------- |
| `TAILSCALE_AUTHKEY` | *(required)* | Tailscale auth key used to join the tailnet. |
| `TAILSCALE_HOSTNAME` | `tailscale-discloud` | Hostname the node will use on the tailnet. |
| `TAILSCALE_STATE` | `/home/discloud/tailscale.state` | Path to the Tailscale state file. |
| `TUNNELS_PATH` | `/home/discloud/tunnels.json` | Path to the tunnel persistence file. |

## Production

### GitHub Releases

Every tagged push (`v*`) triggers the release workflow. It compiles the API binary for `x86_64-unknown-linux-musl` and uploads three archives to GitHub Releases:

| Archive | Contents | Use case |
| ------- | -------- | -------- |
| `release.zip` | `api`, `start.sh`, `public/` | Raw build artifacts without Docker or Discloud files. |
| `deploy-remote.zip` | `Dockerfile`, `discloud.config` | Lightweight deploy package. The container downloads the binary from GitHub Releases at build time. |
| `deploy-static.zip` | `Dockerfile` (patched for local mode), `discloud.config`, `api`, `start.sh`, `public/` | Self-contained deploy package. No network access to GitHub is needed during the Docker build. |

### Dockerfile

The `Dockerfile` supports two build modes controlled by the `BUILD_SOURCE` argument:

| `BUILD_SOURCE` | Behavior |
| -------------- | -------- |
| `remote` (default) | Downloads `release.zip` from GitHub Releases. No local files other than the Dockerfile itself are needed. |
| `local` | Uses `api`, `start.sh`, and `public/` from the build context via `COPY`. |

**Remote mode** (default):

```bash
docker build -t tailscale-discloud .
```

Pin a specific version or point to a different repository:

```bash
docker build --build-arg TUNNEL_MANAGER_VERSION=v0.1.0 -t tailscale-discloud .
docker build --build-arg GITHUB_REPO=your-user/your-fork -t tailscale-discloud .
```

**Local mode** (requires `api`, `start.sh`, and `public/` in the build context):

```bash
docker build --build-arg BUILD_SOURCE=local -t tailscale-discloud .
```

The `mise run package` task automatically produces a `dist/` directory with a patched Dockerfile that defaults to local mode.

### Deploying to Discloud

The easiest way to deploy is to use the official **[Discloud template](https://discloud.com/templates/tailscale-discloud)**. Just click **Host Now**, configure the options to your liking, and deploy — no file downloads required.

Alternatively, you can download one of the ready-made zip files from the [GitHub Releases](https://github.com/jackskelt/tailscale-discloud/releases) page and upload it through the Discloud dashboard or CLI:

- **`deploy-remote.zip`** -- Contains only the `Dockerfile` and `discloud.config`. The container downloads the binary from GitHub Releases at build time. This is the smallest download but requires network access to GitHub during the Docker build.
- **`deploy-static.zip`** -- Contains the compiled binary, entrypoint, static files, `Dockerfile`, and `discloud.config`. No external downloads happen during the Docker build.

For local development builds, use `mise run zip` to produce an equivalent self-contained zip from your own source tree.

```
TYPE=bot
NAME=Tailscale
AVATAR=https://tailscale.com/favicon.png
MAIN=Dockerfile
RAM=100
VLAN=true
```

Refer to [DISCLOUD.md](./DISCLOUD.md) for detailed deployment instructions.

## Development

Local development uses [mise](https://mise.jdx.dev) to manage the Rust toolchain and run project tasks.

### Prerequisites

- [mise](https://mise.jdx.dev) installed and activated in your shell.
- A C linker capable of targeting musl (e.g. `musl-tools` on Debian/Ubuntu).

Install dependencies:

```bash
mise install
```

### Available Tasks

| Command | Description |
| ---------------- | ----------------------------------------------------------- |
| `mise run build` | Compiles the API binary for `x86_64-unknown-linux-musl`. |
| `mise run package` | Runs `build`, then assembles a `dist/` directory with the Dockerfile, binary, entrypoint, static files, and Discloud config. |
| `mise run zip` | Runs `package`, then creates `dist/tailscale-discloud.zip` ready for deployment. |
| `mise run clean` | Removes the `dist/` directory and all Cargo build artifacts. |

The `dist/` directory mirrors the structure expected by Discloud:

```
dist/
  Dockerfile
  discloud.config
  api
  start.sh
  public/
```

### Typical Workflow

1. Make changes to the Rust source in `src/` or the frontend in `public/`.
2. Run `mise run package` to compile and assemble everything into `dist/`.
3. Run `mise run zip` to produce a deployment-ready zip.
4. Upload `dist/release.zip` to Discloud for testing.

## License

This project is licensed under the [GNU General Public License v2.0](LICENSE). You are free to use, modify, and redistribute this software, provided that all derivative works remain open-source under the same license and give appropriate credit to the original author.

## Disclaimer

This project is not affiliated with, endorsed by, or associated with Tailscale Inc. or the Tailscale brand in any way. "Tailscale" is a registered trademark of Tailscale Inc.
