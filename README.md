<p align="center"><img src="public/supergpt.png" alt="FishNineteenGPT Connect" width="96"></p>

# FishNineteenGPT Connect

一个简单的本地 API 配置器。选择中转站、选择 Claude 或 Codex、填写密钥、勾选模型，然后导入。

本仓库是 [SuperGPT Connect](https://github.com/superpilot69/supergpt-connect) 的 **品牌化复刻**：用户可见品牌与默认中转站已改为 FishNineteenGPT，Claude / Codex 本地配置能力保持不变。

**v0.1.6** · [反馈问题](https://github.com/helenaemmaoliviatomas-hub/supergpt-connectAnc/issues) · [MIT 许可](LICENSE)

## 本次品牌化改动

1. **用户可见品牌**：界面、窗口标题、打包产物、说明文案中的 SuperGPT / SuperGPT Connect / `console.supergpt.dev` 全部改为 **FishNineteenGPT**（或 **FishNineteenGPT Connect**）。
2. **默认中转站**：新安装默认站点为 [FishNineteenGPT](https://fishnineteenanh.ccwu.cc)，API 基础地址为 `https://fishnineteenanh.ccwu.cc`，不再默认使用 `console.supergpt.dev` / `api.supergpt.dev`。
3. **功能保留**：仍可配置 Claude 桌面版 / Claude Code（终端与 VS Code）以及 Codex 桌面版与终端版；仍可自行添加其他中转站，一次只使用一个。
4. **开源致谢**：继续采用 MIT；保留 CC Switch 与上游 SuperGPT Connect 来源说明，详见下方「开源与来源」与 [UPSTREAM.md](UPSTREAM.md)。

## 配置步骤

1. **选择中转站**：默认 [FishNineteenGPT](https://fishnineteenanh.ccwu.cc)，也可以添加自己的站点。一次只使用一个。
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

## 本地构建与验证

需要 Node.js 22+（推荐 24）、pnpm 10 与当前稳定版 Rust。macOS 需要 Xcode Command Line Tools；Windows 需要 MSVC Build Tools 和 Windows SDK。

```sh
pnpm install --frozen-lockfile
pnpm check
pnpm test:ui
cargo test --manifest-path src-tauri/Cargo.toml --no-default-features --locked
```

开发运行（会改本机配置，建议先用隔离目录）：

```sh
pnpm dev
```

仅预览界面、不写本机配置：

```sh
pnpm dev:web
```

真实 Codex 联调（仅使用隔离目录、虚构密钥和本机模拟网关）：

```sh
cargo build --manifest-path src-tauri/Cargo.toml --no-default-features --locked --example codex_fixture
python scripts/verify-codex-runtime.py --download-codex
```

隔离测试界面：

```sh
python3 scripts/mock-gateway.py
# 另一终端，debug 构建：
SUPERGPT_CONNECT_TEST_HOME=/absolute/path/to/test-home src-tauri/target/debug/supergpt-connect
```

添加站点 `http://127.0.0.1:18473`，输入虚构 Key `sk-local-fixture-only`。模拟网关只返回模型目录，不调用真实模型。

手动确认品牌与默认站：

1. 打开应用后，标题栏 / 页脚应显示 **FishNineteenGPT Connect**。
2. 第一步默认中转站应为 **FishNineteenGPT**，地址为 `fishnineteenanh.ccwu.cc`。
3. 仍可「添加站点」写入其他 HTTPS 网关，并在 Claude / Codex 之间切换导入。

打包：

```sh
# macOS Apple Silicon ZIP（在 macOS 执行）
rustup target add aarch64-apple-darwin
pnpm build:macos

# Windows x64 免安装 ZIP（在 Windows 执行）
pnpm build:windows
```

产物在 `release/`，文件名以 `FishNineteenGPT-Connect-` 开头。Windows 使用系统 WebView2 Runtime；缺少时从 [微软官网](https://developer.microsoft.com/microsoft-edge/webview2/) 安装。

## 开源与来源

采用 **MIT** 许可。基于 [CC Switch](https://github.com/farion1231/cc-switch) 裁剪配置引擎，并致谢上游 [SuperGPT Connect](https://github.com/superpilot69/supergpt-connect)。保留上游版权与许可；使用 React、Tauri 和 Rust。裁剪范围见 [UPSTREAM.md](UPSTREAM.md)，第三方许可见 [licenses/](licenses/)。本项目独立维护，不是 Claude 或 Codex 的官方客户端。

反馈问题时请说明系统、客户端版本与操作步骤；不要上传 API Key、认证文件或完整恢复记录。
