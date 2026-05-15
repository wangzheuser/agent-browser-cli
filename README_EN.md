<div align="center">

# agent-browser-cli

A browser perception and control CLI for agents, turning a real Chrome session into reusable tab scanning, page JavaScript, Cookie, CDP, and screenshot capabilities.

Browser perception · Page control · Chrome session reuse · CDP · Conditional wait · Agent Skill integration

<p>
  <a href="https://github.com/sleepinginsummer/agent-browser-cli"><img src="https://img.shields.io/badge/CLI-agentbrowsercli-2ea44f" alt="CLI agentbrowsercli"></a>
  <a href="https://github.com/sleepinginsummer/agent-browser-cli/blob/main/LICENSE"><img src="https://img.shields.io/badge/License-MIT-green" alt="License MIT"></a>
  <a href="https://github.com/sleepinginsummer/agent-browser-cli"><img src="https://img.shields.io/badge/sys-win%2Fmac%2Flinux-0078D6?labelColor=0078D6&color=C0C0C0" alt="sys win/mac/linux"></a>
  <a href="https://github.com/sleepinginsummer/agent-browser-cli/releases"><img src="https://img.shields.io/badge/release-v0.2.7-blue" alt="release v0.2.7"></a>
  <a href="https://github.com/sleepinginsummer/agent-browser-cli/pulls"><img src="https://img.shields.io/badge/PRs-welcome-brightgreen" alt="PRs welcome"></a>
</p>

[AI One-Line Install](#ai-one-line-install) · [Manual Installation](#manual-installation) · [Chrome Extension](#chrome-extension) · [Update](#update) · [Uninstall](#uninstall) · [Friendly Links](#friendly-links)

[中文](README.md) | English

</div>

`agent-browser-cli` is a browser perception and control tool for agents. It connects to the user's real Chrome browser through a Chrome extension, preserving login state and cookies while providing tab scanning, page JavaScript execution, cookie reading, CDP control, screenshots, file uploads, dropdown clicks, and related capabilities.

This project is not Selenium or Playwright. It is better suited for helping agents read pages accurately and perform actions inside an existing browser session.

## Project Info

- Current version: `0.2.7`
- Supported platforms: Windows (including WSL) / Mac / Linux
- Browser: Chrome, with the `assets/tmwd_cdp_bridge` extension loaded
- Linux prerequisite: the local Chrome / Chromium build must support loading extensions
- WSL prerequisite: use `WSL 2.0.0+`, and preferably enable `networkingMode=mirrored` on Windows `11 22H2+` so WSL can reach the host Chrome bridge service on `localhost`

## Acknowledgements

The browser control capability in this project was extracted and adapted from the Web toolchain in [GenericAgent](https://github.com/lsdefine/GenericAgent), including ideas and implementation around `TMWebDriver`, `simphtml`, and the `tmwd_cdp_bridge` extension.

Thanks to the GenericAgent project for the browser bridge, page simplification, CDP control, and practical SOPs. This repository reorganizes and enhances that work for standalone usage and CLI invocation.

## AI One-Line Install

```text
Please read https://github.com/sleepinginsummer/agent-browser-cli/blob/main/AI_INSTALL.md, follow the instructions to install the CLI, load the Chrome extension, and add `skills/agent-browser-cli/SKILL.md`.
```

## Improvements

- Extracted browser control capability from GenericAgent and exposed it as a CLI for Codex, Claude Code, and OpenCode. The GenericAgent browser extension can be reused and does not need to be reinstalled.
- Avoids reinitializing the browser connection for every command.
- Adds a startup lock to avoid repeated low-level port binding when multiple CLI commands start concurrently.
- Adds the skill `skills/agent-browser-cli/SKILL.md` for AI usage reference.
- Includes several optimizations to reduce command execution time.
- Rust implementation for the CLI side.

## Performance Reference

The following numbers are measured with the long-lived service already running and the Chrome extension already connected. Actual latency depends on page complexity, network conditions, Chrome state, and response size.

| Operation | Reference Latency |
| --- | --- |
| Open a Baidu tab | About `0.10s` |
| Inject JS to enter a keyword and submit search | About `0.27s` |
| Open Baidu and search “小猫” end-to-end | About `0.37s` |
| `scan --tab --text-only` to read page text | About `0.04-0.12s` |
| `exec 'return document.title'` for simple JS | About `0.04-0.12s` |
| `exec 'return document.body.innerText'` to read body text | Mostly `0.04-0.05s`, occasional `0.30s` |
| Query DOM link lists | About `0.27-0.36s` |
| `exec --monitor` page-change summary | About `0.72-0.88s` |

Rule of thumb: normal page reads and simple JS injection are around the `50ms` level; complex DOM queries depend on page structure and returned data size, commonly around `300ms`; `--monitor` adds page-change summary work and is usually close to `0.8s`.

Reference comparison with the original Python call chain:

| Item | Python Version | Rust CLI Version |
| --- | --- | --- |
| Startup model | Each call is more likely to pay for Python process startup, module loading, and connection initialization | CLI commands reuse the long-lived service and avoid repeated browser connection initialization |
| Simple page read / JS injection | Usually more affected by process startup and the Python call chain, so latency is less stable | Commonly `0.04-0.12s` |
| Repeated calls | Overhead is more visible across many short commands | Better suited for high-frequency Agent calls |

This comparison is intended to describe the performance trend caused by the architecture difference. Actual latency still depends on page complexity, Chrome state, and response size.

## Layout

```text
.
├── Cargo.toml                    # Rust crate config
├── src/                          # Rust CLI / daemon / bridge
├── assets/tmwd_cdp_bridge/       # Chrome MV3 extension
├── assets/simphtml_opt.js        # Page simplification script
├── assets/simphtml_find_list.js  # List detection script
├── npm/                          # npm launcher scripts
└── skills/agent-browser-cli/     # skill
```

## Manual Installation

### npm

```bash
npm install -g @sleepinsummer/agent-browser-cli
agent-browser-cli tabs
```

### Build From Source

```bash
cargo build --release
./target/release/agent-browser-cli tabs
```


## Chrome Extension

Recommended: download `chrome-extensions.zip` from the [latest Release](https://github.com/sleepinginsummer/agent-browser-cli/releases/latest), extract it, open `chrome://extensions` in Chrome, enable `Developer mode`, click `Load unpacked`, and select the extracted `tmwd_cdp_bridge` directory.

When building from local source, you can also load this extension directory directly:

```text
assets/tmwd_cdp_bridge
```

Chrome needs at least one normal web page tab open. Do not leave it only on `about:blank` or `chrome://` pages.

After the extension is connected, a Chrome extension tip badge appears on the right side of the page. The badge position is draggable and expands on hover. It auto-hides after 10 seconds without commands, and you can also click `Hide for this session` to hide it manually. Manual hiding lasts for the current service connection cycle and resets after the service disconnects after about 300 seconds and reconnects.

## Quick Check

```bash
agent-browser-cli tabs
agent-browser-cli open https://www.baidu.com
```

On success, it returns:

```json
{
  "ok": true,
  "result": {
    "status": "success",
    "metadata": {
      "tabs_count": 1
    }
  }
}
```

## Common Commands

The README only keeps the quick entry point. For the full command list and browser operation SOP, see [skills/agent-browser-cli/SKILL.md](./skills/agent-browser-cli/SKILL.md).

```bash
agent-browser-cli tabs
```

## Update

```bash
git pull
cargo build --release
./target/release/agent-browser-cli restart
```

If the Chrome extension has updates, reload the `assets/tmwd_cdp_bridge` extension in `chrome://extensions`.

Current extension bridge identifier:

```js
const TID = '__agent_browser_cli_bridge_26c9f1';
```

If you installed the skill into a global Codex/Agent directory, copy it again after updating:

```bash
mkdir -p ~/.agents/skills/agent-browser-cli
cp skills/agent-browser-cli/SKILL.md ~/.agents/skills/agent-browser-cli/SKILL.md
```

## Uninstall

Stop the long-lived service first:

```bash
agent-browser-cli stop
```

Then clean up as needed:

```bash
rm -f .agent-browser-cli.log .agent-browser-cli.lock
rm -rf ~/.agents/skills/agent-browser-cli
```

Finally, remove the `TMWD CDP Bridge` extension from Chrome's extension management page, or remove the loaded `assets/tmwd_cdp_bridge` extension configuration.

## Ports

- `18765`: underlying `TMWebDriver` WebSocket, used by the Chrome extension.
- `18767`: outer `agent-browser-cli` HTTP service, used by the CLI to reuse the session.

## Friendly Links

- [LINUX DO - A New Ideal Community](https://linux.do/)
- [GenericAgent--复旦团队研发|仅仅~3K行代码 Self-Evolving Agent](https://github.com/lsdefine/GenericAgent/tree/main)

## License

MIT License. See [LICENSE](./LICENSE).
