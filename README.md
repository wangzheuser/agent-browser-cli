<div align="center">

# agent-browser-cli

面向 Agent 的浏览器感知与控制 CLI，把真实 Chrome 会话变成可复用的标签页扫描、页面 JS、Cookie、CDP 和截图能力。

浏览器感知 · 页面控制 · Chrome 登录态复用 · CDP · 条件等待 · Agent Skill 集成

<p>
  <a href="https://github.com/sleepinginsummer/agent-browser-cli"><img src="https://img.shields.io/badge/CLI-agentbrowsercli-2ea44f" alt="CLI agentbrowsercli"></a>
  <a href="https://github.com/sleepinginsummer/agent-browser-cli/blob/main/LICENSE"><img src="https://img.shields.io/badge/License-MIT-green" alt="License MIT"></a>
  <a href="https://github.com/sleepinginsummer/agent-browser-cli"><img src="https://img.shields.io/badge/Windows-MacOS-0078D6?labelColor=0078D6&color=C0C0C0" alt="Windows/MacOS"></a>
  <a href="https://github.com/sleepinginsummer/agent-browser-cli/releases"><img src="https://img.shields.io/badge/release-v0.2.1-blue" alt="release v0.2.1"></a>
  <a href="https://github.com/sleepinginsummer/agent-browser-cli/pulls"><img src="https://img.shields.io/badge/PRs-welcome-brightgreen" alt="PRs welcome"></a>
</p>

[AI 一句话安装](#ai-一句话安装) · [手动安装](#手动安装) · [Chrome 扩展](#chrome-扩展) · [更新](#更新) · [卸载](#卸载) · [友情链接](#友情链接)

中文 | [English](README_EN.md)

</div>

`agent-browser-cli` 是一个面向 Agent 的浏览器感知与控制工具。它通过 Chrome 扩展连接用户真实浏览器，保留登录态和 Cookie，提供标签页扫描、页面 JS 执行、Cookie 读取、CDP 控制、截图、文件上传、下拉框点击等能力。

本项目不是 Selenium / Playwright。它更适合在已有浏览器会话中辅助 Agent 精确读取页面和执行操作。

## 项目信息

- 当前版本：`0.2.1`
- 支持平台：Windows、macOS
- 浏览器：Chrome / Chromium，需加载 `assets/tmwd_cdp_bridge`

## 致谢

本项目的浏览器控制能力提取并改造自 [GenericAgent](https://github.com/lsdefine/GenericAgent) 项目中的 Web 工具链，包括 `TMWebDriver`、`simphtml` 和 `tmwd_cdp_bridge` 扩展相关思路与实现。

感谢 GenericAgent 项目提供的浏览器桥接、页面简化、CDP 控制和实践 SOP。本仓库在此基础上做了面向独立使用和 CLI 调用的整理与增强。

## AI 一句话安装

```text
请阅读 https://github.com/sleepinginsummer/agent-browser-cli/blob/main/AI_INSTALL.md，按说明安装 CLI、加载 Chrome 扩展，并添加 `skills/agent-browser-cli/SKILL.md`。
```

## 改进内容

- 从 GenericAgent 中拆出浏览器控制能力，使用cli 提供给codex、claude code、opencode使用。GenericAgent浏览器插件不需要重新安装，可以共用同一个插件
- 避免每次命令都重新初始化浏览器连接。
- 新增启动锁，避免多个 CLI 并发启动时重复绑定底层端口。
- 增加skill：`skills/agent-browser-cli/SKILL.md`，提供ai参考使用。
- 若干优化，缩短命令执行时间
- rust实现cli端

## 性能参考

以下为常驻服务已启动、Chrome 扩展已连接时的实测参考，实际耗时会受页面复杂度、网络、Chrome 状态和返回数据量影响。

| 操作 | 参考耗时 |
| --- | --- |
| 打开百度标签页 | 约 `0.10s` |
| 注入 JS 输入关键词并点击搜索 | 约 `0.27s` |
| 打开百度并搜索“小猫”合计 | 约 `0.37s` |
| `scan --tab --text-only` 读取页面文本 | 约 `0.04-0.12s` |
| `exec 'return document.title'` 注入简单 JS | 约 `0.04-0.12s` |
| `exec 'return document.body.innerText'` 读取正文 | 多数 `0.04-0.05s`，偶发约 `0.30s` |
| 查询 DOM 链接列表 | 约 `0.27-0.36s` |
| `exec --monitor` 页面变化摘要 | 约 `0.72-0.88s` |

一般判断：普通读页面和简单 JS 注入是 `50ms` 级；复杂 DOM 查询主要取决于页面结构和返回数据量，常见约 `300ms`；`--monitor` 会额外生成页面变化摘要，通常接近 `0.8s`。

## 目录结构

```text
.
├── Cargo.toml                    # Rust 工程配置
├── src/                          # Rust CLI / 常驻服务 / bridge
├── assets/tmwd_cdp_bridge/       # Chrome MV3 扩展
├── assets/simphtml_opt.js        # 页面简化脚本
├── assets/simphtml_find_list.js  # 列表识别脚本
├── npm/                          # npm 启动脚本
└── skills/agent-browser-cli/     # skill
```

## 手动安装

### npm 安装

```bash
npm install -g @sleepinsummer/agent-browser-cli
agent-browser-cli tabs
```

### 本地源码构建

```bash
cargo build --release
./target/release/agent-browser-cli tabs
```


## Chrome 扩展

加载扩展目录：

```text
assets/tmwd_cdp_bridge
```

Chrome 需要至少打开一个正常网页标签页，不要只停留在 `about:blank` 或 `chrome://` 页面。

## 快速自检

```bash
agent-browser-cli tabs
agent-browser-cli open https://www.baidu.com
```

成功时会返回：

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

## 常用命令

README 只保留快速入口；完整命令和浏览器操作 SOP 见 [skills/agent-browser-cli/SKILL.md](./skills/agent-browser-cli/SKILL.md)。

```bash
agent-browser-cli tabs
```

## 更新

```bash
git pull
cargo build --release
./target/release/agent-browser-cli restart
```

如果 Chrome 扩展有更新，在 `chrome://extensions` 中重新加载 `assets/tmwd_cdp_bridge` 扩展。

当前扩展配置标识为：

```js
const TID = '__agent_browser_cli_bridge_26c9f1';
```

如果你把 skill 安装到了 Codex/Agent 的全局目录，更新后同步复制：

```bash
mkdir -p ~/.agents/skills/agent-browser-cli
cp skills/agent-browser-cli/SKILL.md ~/.agents/skills/agent-browser-cli/SKILL.md
```

## 卸载

先停止常驻服务：

```bash
agent-browser-cli stop
```

然后按需清理：

```bash
rm -f .agent-browser-cli.log .agent-browser-cli.lock
rm -rf ~/.agents/skills/agent-browser-cli
```

最后在 Chrome 扩展管理页中移除 `TMWD CDP Bridge` 扩展，或删除已加载的 `assets/tmwd_cdp_bridge` 扩展配置。

## 端口

- `18765`：底层 `TMWebDriver` WebSocket，Chrome 扩展连接使用。
- `18767`：外层 `agent-browser-cli` HTTP 服务，供 CLI 复用会话。

## 友情链接

- [LINUX DO - 新的理想型社区](https://linux.do/)

## 许可证

MIT License. See [LICENSE](./LICENSE).
