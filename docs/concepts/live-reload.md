# Live reload

Zorto's dev server watches your files, rebuilds automatically, and refreshes the browser — no manual reload needed.

## Usage

```bash
zorto preview
```

This starts a local server (default port 1111). Add `--open` to automatically open your site in the browser.

## What gets watched

The server monitors all files that affect your site:

| Directory | Triggers rebuild on |
|-----------|-------------------|
| `content/` | Any `.md` file change |
| `templates/` | Any `.html` file change |
| `sass/` | Any `.scss` file change |
| `static/` | Any file change (copied to output) |
| `config.toml` | Configuration change |

## How it works

{{ flow(steps="Save:Edit a file|Detect:Filesystem event|Rebuild:Site recompiles (~100ms)|Push:SSE event sent|Refresh:Browser reloads", caption="Uses server-sent events (SSE). No browser extension required.") }}

1. You save a file.
2. Zorto detects the change via filesystem events.
3. The site rebuilds (typically under 100ms).
4. The server pushes a reload event over [SSE](https://developer.mozilla.org/en-US/docs/Web/API/Server-sent_events) (server-sent events).
5. A small injected script in every page listens for the event and triggers a browser refresh.

## Options

```bash
zorto preview --port 8080         # custom port
zorto preview --open              # auto-open browser
zorto preview --interface 0.0.0.0 # expose to local network
zorto --no-exec preview           # skip executable code blocks
```

## Network access

By default, the server binds to `127.0.0.1` (localhost only). Use `--interface 0.0.0.0` to make the preview accessible from other devices on your network — useful for testing on phones or tablets.

## Further reading

- [Fast](fast.md) — why build times are under 100ms
- [CLI reference](../reference/cli.md) — all preview server options
- [Troubleshooting](../how-to/troubleshooting.md) — preview server issues and fixes
