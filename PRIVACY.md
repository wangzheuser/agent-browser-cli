# 隐私政策 / Privacy Policy

Agent Browser CLI Bridge（下称"本扩展"）

最后更新：2026-10-29

## 本扩展是什么

本扩展是 `agent-browser-cli` 命令行工具的浏览器端代理。它把浏览器控件（标签页、Cookie、页面脚本、调试协议）暴露给你本机运行的 CLI，让你的 Agent 能操作你**已经登录**的 Chrome。

## 数据在哪里流转

**所有通信只发生在本机。** 本扩展只与本机 `127.0.0.1` 上的 CLI 守护进程建立 WebSocket 连接（默认端口 18765），不连接任何远程服务器，不上传任何数据，不含统计、埋点或广告 SDK。

CLI 守护进程本身由你在本机启动和停止；本扩展不包含任何云服务地址。

## 本扩展能访问什么

为了让 CLI 完成浏览器操作，本扩展在你授权后具备下列能力。这些能力只在**你本机 CLI 主动发起命令时**使用，扩展不会自行读取或上传这些信息：

| 能力 | 用于 |
| --- | --- |
| 读取与设置 Cookie | 复用你已登录的会话，执行需要登录的操作 |
| 在任意网站执行脚本 | 按 CLI 指令点击、填表、读取页面内容 |
| 标签页与标签组 | 打开、切换、关闭、分组管理标签页 |
| 调试协议（`chrome.debugger`） | 执行页面交互、截图、监听网络请求与控制台日志 |
| 扩展管理 | 按 CLI 指令列出、启用、禁用其他扩展 |
| 内容设置 | 按 CLI 指令读取或修改网站级内容权限 |

`<all_urls>` 主机权限是必需的：CLI 的用途就是操作你指定的任意网页；扩展不会主动访问你没有让它访问的页面。

## 本扩展不做什么

- 不把 Cookie、页面内容、浏览记录或调试数据发送到本机以外的任何地方
- 不含广告、不投放广告、不出售数据
- 不修改搜索引擎、不劫持新标签页、不注入任何第三方脚本
- 不包含远程代码：所有 JS 都打包在扩展内，不从网络加载或执行

## 关闭与删除

- 在扩展弹窗中可随时断开与 CLI 的连接
- 停掉本地 CLI 守护进程后，扩展不再收到任何指令
- 卸载扩展会删除其本地设置（端口、Profile 标识）

## 联系方式

问题或安全报告请提交到：
https://github.com/sleepinginsummer/agent-browser-cli/issues

---

# Privacy Policy (English)

Agent Browser CLI Bridge is the browser-side agent of the `agent-browser-cli` command line tool. It exposes browser controls (tabs, cookies, page scripts, DevTools protocol) to a CLI running **on your own machine**.

**All traffic stays on the local machine.** The extension only opens a WebSocket connection to a local daemon on `127.0.0.1` (default port 18765). It contacts no remote server, uploads no data, and contains no analytics, tracking or advertising SDK.

The listed capabilities (cookies, script injection, tabs/tab groups, `chrome.debugger`, extension management, content settings) are used only when your local CLI issues a command. The `<all_urls>` host permission is required because the tool exists to operate arbitrary web pages you point it at. No data leaves your device, and no remote code is loaded or executed — all JavaScript ships inside the package.

Uninstall the extension to remove its local settings.
