# AI 安装说明

把下面这段话发给 AI，让 AI 在你的本机环境里完成安装、配置 skill 和验证。

```text
请帮我安装 agent-browser-cli：https://github.com/sleepinginsummer/agent-browser-cli

要求：
1. 优先使用 npm 安装：npm install -g @sleepinsummer/agent-browser-cli。
2. 指导我在 Chrome 中加载 assets/tmwd_cdp_bridge 解压扩展。
3. 如果之前已经加载过扩展，必须在 chrome://extensions 里重新加载 `assets/tmwd_cdp_bridge`，确保最新 `config.js` 和 `background.js` 生效。
4. 将 skills/agent-browser-cli/SKILL.md 安装到当前 AI 可识别的 skills 目录。
5. 执行 agent-browser-cli tabs、open 和 status 验证可用。
6. 如果 npm 平台包暂不支持当前系统，再回退到源码构建：cargo build --release。
```

## 1. 安装 CLI

优先使用 npm 全局安装：

```bash
npm install -g @sleepinsummer/agent-browser-cli
agent-browser-cli --help
```

当前 npm 包按平台安装原生二进制：

```text
@sleepinsummer/agent-browser-cli
@sleepinsummer/agent-browser-cli-darwin-arm64
@sleepinsummer/agent-browser-cli-darwin-x64
@sleepinsummer/agent-browser-cli-win32-x64
```

如果当前平台包暂未发布或安装失败，使用源码构建：

```bash
git clone https://github.com/sleepinginsummer/agent-browser-cli.git
cd agent-browser-cli
cargo build --release
./target/release/agent-browser-cli --help
```

## 2. 加载 Chrome 扩展

如果使用 npm 安装，需要先下载或克隆仓库，用于加载扩展和安装 skill：

```bash
git clone https://github.com/sleepinginsummer/agent-browser-cli.git
cd agent-browser-cli
```

在 Chrome 打开：

```text
chrome://extensions
```

开启“开发者模式”，加载已解压扩展目录：

```text
assets/tmwd_cdp_bridge
```

如果之前已经安装过旧版 GenericAgent 的 `tmwd_cdp_bridge` 扩展，可以继续使用同协议旧扩展；但建议加载当前仓库的 `assets/tmwd_cdp_bridge` 并点击“重新加载”。

当前扩展配置应包含：

```js
const TID = '__agent_browser_cli_bridge_26c9f1';
```

Chrome 至少需要打开一个正常网页标签页，不要只停留在 `about:blank` 或 `chrome://` 页面。

## 3. 安装 skill

将仓库中的 `skills/agent-browser-cli/SKILL.md` 安装到 AI 使用的 skills 目录。

通用目录示例：

```bash
mkdir -p ~/.agents/skills/agent-browser-cli
cp skills/agent-browser-cli/SKILL.md ~/.agents/skills/agent-browser-cli/SKILL.md
```

Codex 默认目录示例：

```bash
mkdir -p ~/.codex/skills/agent-browser-cli
cp skills/agent-browser-cli/SKILL.md ~/.codex/skills/agent-browser-cli/SKILL.md
```

如果 AI 使用其它 skills 目录，将 `SKILL.md` 复制到对应的 `agent-browser-cli/SKILL.md`。

## 4. 验证

```bash
agent-browser-cli tabs
agent-browser-cli open https://www.baidu.com
agent-browser-cli status
```

成功时，`tabs` 会返回 `ok: true`，并包含当前 Chrome 标签页数量。
`open` 应能原生新开标签页，不应使用 `exec --monitor` 或 `window.open` 代替。

如果常驻服务需要重载最新代码：

```bash
agent-browser-cli restart
```

## 5. 使用入口

拿到标签页 ID 后，可以执行：

```bash
agent-browser-cli scan --tab <tabId> --text-only
agent-browser-cli exec --tab <tabId> 'return document.title'
```

完整命令和浏览器操作 SOP 见：

```text
skills/agent-browser-cli/SKILL.md
```

