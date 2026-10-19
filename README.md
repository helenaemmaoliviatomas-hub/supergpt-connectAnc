<p align="center"><img src="public/supergpt.png" alt="SuperGPT Connect" width="96"></p>

# SuperGPT Connect

一个简单的本地 API 配置器。选择中转站、选择 Claude 或 Codex、填写密钥、勾选模型，然后导入。

**v0.1.6** · [下载](https://github.com/superpilot69/supergpt-connect/releases/tag/v0.1.6) · [反馈问题](https://github.com/superpilot69/supergpt-connect/issues) · [MIT 许可](LICENSE)

## 下载与使用

| 系统 | 下载 | 打开方式 |
| --- | --- | --- |
| Windows 10 / 11 · x64 | [Windows 免安装 ZIP](https://github.com/superpilot69/supergpt-connect/releases/download/v0.1.6/SuperGPT-Connect-0.1.6-Windows-x64-portable.zip) | 全部解压，双击 `SuperGPT Connect.exe` |
| macOS · Apple Silicon | [Mac ZIP](https://github.com/superpilot69/supergpt-connect/releases/download/v0.1.6/SuperGPT-Connect-0.1.6-macOS-arm64.zip) | 解压后打开 `.app`，也可拖入「应用程序」 |

本页下载使用 [GitHub Releases](https://github.com/superpilot69/supergpt-connect/releases/tag/v0.1.6)。[SHA-256 校验值](https://github.com/superpilot69/supergpt-connect/releases/download/v0.1.6/SHA256SUMS.txt) 也在同一版本下。Mac 包仅支持 M 系列芯片，不支持 Intel Mac。

当前版本通过 macOS 与 Windows 原生自动测试，并使用真实 Codex app-server 验证图片发送、上下文、思考强度、重复导入与恢复。详细范围见 [验证记录](VERIFICATION.md)。

运行时无需安装 Node.js 或 Rust。Windows 使用系统 WebView2 Runtime；缺少时，按应用提示从 [微软官网](https://developer.microsoft.com/microsoft-edge/webview2/) 安装。Windows 包未进行发布者签名；Mac 包使用 ad-hoc 签名，尚未完成 Apple Developer ID 签名与公证。校验值见 Release 的 `SHA256SUMS.txt`。

### 下载与打开提示

浏览器可能提示文件不常下载，macOS 可能提示无法验证开发者。这些检查与功能测试不同；当前发布包的签名状态见上方说明。请保留系统保护设置，遇到拦截时可参考 [Chrome 下载说明](https://support.google.com/chrome/answer/6261569?hl=zh-Hans)，或审阅源码后自行构建。

SHA-256 用于核对下载完整性，不代表安全扫描或系统认证。反馈时请提供提示原文，不要附上 API Key。

### 更新已有 Codex 配置

旧版本 Connect 导入 Codex 后，可能出现可用上下文较短或模型被误标为仅文字、无法添加图片。0.1.4 起修复图片能力导入，0.1.6 保留该修复，请使用当前版本重新获取模型并导入，应用修复并保留已有的上下文与思考设置。

**下载新版 → 重新获取模型并导入 Codex → 完全退出 Codex 再打开 → 新建本地会话。** 只替换连接器程序不会更新已写入的模型目录，也不能还原旧会话中已经压缩掉的内容。

GPT‑6.1 Sol 按发布的 Codex 目录使用默认上下文 272,000、最大可配置容量 872,000；保留已有 `model_context_window` 与 `model_auto_compact_token_limit`。例如原本设置 500,000 窗口时，修复后 Codex 可显示 475,000 的有效容量。不会把所有用户、所有模型统一设成 500,000。

### 配置步骤

1. **选择中转站**：默认 [SuperGPT](https://console.supergpt.dev)，也可以添加自己的站点。一次只使用一个。
2. **选择客户端**：选择 Claude 或 Codex，再选择桌面版、终端版或 VS Code 扩展。
3. **输入 API Key**：点击获取可用模型，读取站点的 `/v1/models`。
4. **勾选模型并导入**：选择需要的模型，指定默认模型，点击导入。

导入后请完全退出目标客户端，再重新打开并新建会话。Claude 在 Windows 上可能留在系统托盘，需要从托盘菜单退出。

需要撤回时，点击右上角 **恢复原配置**，选择客户端，再点恢复。多个站点之间反复导入时，仍保留第一次导入前的恢复点。

## 支持的客户端

| 客户端 | 配置方式 |
| --- | --- |
| Claude 桌面版 | 第三方推理模式的独立网关 profile，用于 Chat、Cowork 与本地 Code |
| Claude Code 终端版 | 用户 `settings.json` 中的 API、默认模型与可用模型菜单 |
| Claude Code · VS Code | 默认用户的扩展设置，并同步 Claude Code 的模型设置 |
| Codex 桌面版与终端版 | 共享用户配置、API Key 认证与所选模型目录 |

尊重 `CLAUDE_CONFIG_DIR` 与 `CODEX_HOME`，也读取 VS Code 扩展单独指定的绝对 `CLAUDE_CONFIG_DIR`；导入与恢复始终使用同一目录。Windows 版配置 Windows 本机客户端；WSL、Remote SSH、自定义 VS Code Profile 需要在对应环境配置。

适配范围是支持本地 API 配置的客户端版本，不包含 Claude 网页版、手机端及云端任务。Claude Code 早于 2.1.242 不支持完整自定义模型菜单；Claude 桌面版需要支持第三方推理模式。受组织管理的 Claude provider、已启用顶层自定义 profile 的 Codex 会阻止本地导入。

中转站须提供 Bearer 认证的 `/v1/models`，并支持目标客户端的 Messages 或 Responses 协议。**读取模型列表、写入配置不等于已验证对话与工具调用。** 导入后需在目标客户端实际确认。测试范围见 [VERIFICATION.md](VERIFICATION.md)。

## 配置与数据

- API Key 不进入站点列表、浏览器存储或日志。获取目录后，仅在当前进程内保存，导入成功后清空；导入时会写入所选客户端需要的本地认证配置。
- 保留 MCP、权限、hooks、无关配置，以及支持范围内的 TOML / JSONC 注释。
- 写入前自动备份，使用文件锁、原子替换与中断恢复，避免部分配置被覆盖。
- 导入后若又修改了配置，恢复前会要求确认，同时保留当前内容的副本。
- 没有恢复记录时不会重置客户端。恢复的是本机文件，不能让服务端失效的登录令牌重新有效。
- 恢复记录位于用户目录的 `.supergpt-connect`，可能含有原凭据，请不要公开上传。macOS / Linux 新写入的配置文件权限为 `0600`。

Codex 的外部模型目录会替换内置目录。网关提供能力元数据时优先采用；仅提供模型名称时，已知模型按公开的 Codex 目录补全思考档位和上下文参数。未知别名可采用本机 Codex 缓存中同名模型的容量，或同一站点现有目录中的手工修正；仍无法确认容量时，导入会停止并提示，客户端文件保持原样。视觉等能力仍需网关明确提供。

## 开发与构建

需要 Node.js 22+（推荐 24）、pnpm 10 与当前稳定版 Rust。macOS 需要 Xcode Command Line Tools；Windows 需要 MSVC Build Tools 和 Windows SDK。

```sh
pnpm install --frozen-lockfile
pnpm dev
pnpm check
pnpm test:ui
cargo test --manifest-path src-tauri/Cargo.toml --no-default-features --locked
# 真实 Codex 联调：仅使用隔离目录、虚构密钥和本机模拟网关
cargo build --manifest-path src-tauri/Cargo.toml --no-default-features --locked --example codex_fixture
python scripts/verify-codex-runtime.py --download-codex
```

[跨平台 CI](https://github.com/superpilot69/supergpt-connect/actions/workflows/verify.yml) 在 macOS 和 Windows 原生执行测试、Codex 联调及打包；运行报告与构建包保存在对应任务的 artifacts 中。

`pnpm dev:web` 只预览界面，不会修改本机配置。打包：

```sh
# macOS Apple Silicon ZIP（在 macOS 执行）
rustup target add aarch64-apple-darwin
pnpm build:macos

# Windows x64 免安装 ZIP（在 Windows 执行）
pnpm build:windows
```

产物在 `release/`。Windows 脚本跳过安装器，校验 EXE 架构、版本、启动权限与 DLL 依赖；CRT 和 WebView2Loader 静态链接，不要求用户另装 VC++ Redistributable。Mac 脚本验证应用版本、arm64 架构与 ad-hoc 签名。

也可在 macOS 交叉构建 Windows 包：

```sh
brew install llvm lld
cargo install --locked cargo-xwin
rustup target add x86_64-pc-windows-msvc
PATH="$(brew --prefix llvm)/bin:$PATH" XWIN_ARCH=x86_64 pnpm build:windows
```

`.cargo/config.toml` 控制完整静态 CRT 链接；`tauri.windows.conf.json` 关闭 Tauri 自身的混合 CRT 覆盖以避免冲突。

实际发布使用已启用的 `.github/workflows/verify.yml`。`ci/build.yml` 仅保留为早期参考模板。

## 隔离测试

Rust 测试使用临时用户目录和虚构密钥。原生 debug 界面可使用独立测试目录：

```sh
python3 scripts/mock-gateway.py
# 在另一终端中，使用已构建的 debug 应用：
SUPERGPT_CONNECT_TEST_HOME=/absolute/path/to/test-home src-tauri/target/debug/supergpt-connect
```

添加站点 `http://127.0.0.1:18473`，输入虚构 Key `sk-local-fixture-only`。模拟网关只返回模型目录，不调用真实模型。测试目录覆盖仅在 debug 构建生效，界面会显示测试模式。

反馈问题时请说明系统、客户端版本与操作步骤；不要上传 API Key、认证文件或完整恢复记录。

## 开源与来源

采用 MIT 许可，基于 [CC Switch](https://github.com/farion1231/cc-switch) 裁剪配置引擎，保留上游版权与许可。使用 React、Tauri 和 Rust；来源与裁剪范围见 [UPSTREAM.md](UPSTREAM.md)，第三方许可见 [licenses/](licenses/)。本项目独立维护，不是 Claude 或 Codex 的官方客户端。


### MCP 工具按需加载

Claude 导入前显示当前密钥对应线路的能力和本机冲突。默认自动：只有所选模型全部通过服务端验证才开启；没有能力接口、验证过期或某模型未验证时关闭并说明可能增加初始上下文。读取能力仅发送认证 GET，不发送收费探测请求。可选择关闭，或手动开启未验证线路。已确认不支持的线路不能强制开启。切换线路后需要重新导入并重启客户端。

终端/VS Code 使用 ENABLE_TOOL_SEARCH，Claude 桌面版使用 toolSearchEnabled。禁用实验功能、ToolSearch 禁止规则、旧版本、项目或组织策略可能影响实际行为；Connect 保留这些设置并展示能检测到的冲突。alwaysLoad 工具仍会提前加载。

### 用途模型与兼容选项（0.1.6）

Claude Code 的高级设置可分别选择 Haiku 后台任务、Sonnet / Opus / Fable 别名和子代理模型，只能选择本次已勾选的模型。自动模式优先采用唯一匹配的同系列模型；匹配不明确时沿用默认模型并显示费用提示，不猜测哪个别名更便宜。子代理默认遵循自身模型设置和当前会话，不再固定为导入时的默认模型。

CLI 和 VS Code 共用同一份连接规则：清空会抢占路由的常见旧 shell 凭据和云平台开关，第三方网关设置 `CLAUDE_CODE_AUTO_MODE_SERVER=0`，不改权限审批模式。VS Code 同时设置 `claudeCode.disableLoginPrompt=true`。原有 Artifact 开关默认保留，也可为拒绝该工具格式的线路单独禁用；不连带关闭 MCP 按需加载。

Codex 使用服务商表内的 `experimental_bearer_token`，不覆盖 `auth.json` 或改动 keyring 偏好。旧版本 Connect 已保存的恢复点仍可恢复。旧配置占用 `openai`、`ollama`、`lmstudio` 保留名称的表会备份后移除，其他服务商表保留。联网搜索可保留原值、关闭、缓存或实时；首次默认关闭，开启需要线路支持且可能产生额外费用。MCP 工具搜索与联网搜索是不同功能。

Windows Claude 桌面配置目录带后缀时会识别唯一目录，多个候选会提示歧义。桌面旧 `enterpriseConfig` 中的本地网关字段会迁移，其他策略、MCP 和原 profile 保留。旧版本、旧容量/思考偏好、项目与组织覆盖仍需在新会话 `/status`、`/context` 中核对。

[CC Switch 全面对照记录](COMPATIBILITY.md) 列出本次修复、保留行为、适用边界和回归方法。
