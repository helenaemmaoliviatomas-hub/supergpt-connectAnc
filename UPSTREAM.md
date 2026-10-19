# CC Switch 代码来源

- 项目：https://github.com/farion1231/cc-switch
- 固定提交：`8596a233b2373226a0307d5f80ec0954d5fa162f`
- 上游 package.json 版本：4.0.3
- 原作者：Jason Young
- 许可：MIT，完整声明见 `licenses/CC-Switch-MIT.txt`；应用包内也附带此文件。

本项目提取 CC Switch 的配置修改引擎，保留 React + Tauri 技术栈，重写成四步连接器。

| 本项目 | 上游路径 | 调整 |
| --- | --- | --- |
| `src-tauri/src/patch/json.rs` | `src-tauri/src/live/patch/json.rs` | 保留 JSON 补丁与格式处理，调整测试模块路径 |
| `src-tauri/src/patch/toml.rs` | `src-tauri/src/live/patch/toml.rs` | 保留 TOML 注释与结构补丁，调整测试模块路径 |
| `src-tauri/src/patch/mod.rs` | `src-tauri/src/live/patch/mod.rs` | 移除数据库、整文件引擎及 AppError 依赖，保留补丁接口与解析错误 |
| `src-tauri/src/provider_fields.rs` | `src-tauri/src/live/floor.rs` | 提取 Claude 和 Codex 的服务商字段定义 |
| `src-tauri/src/resources/codex_native_responses_template.json` | 同路径 | 复制 Responses 目录基础模板；按确切模型 ID 补全容量与思考档位，网关元数据优先 |
| `src-tauri/src/adapter.rs` 的 Claude 模型菜单 | `src-tauri/src/mode/controller.rs` | 参考上游 `modelPicker` 自定义选项结构，按用户勾选结果写入 |
| `src-tauri/src/desktop_config.rs` | `src-tauri/src/claude_desktop_config.rs` | 采用 3P profile + metadata + deploymentMode 布局；使用独立 profile ID，不复制代理服务和放宽网络权限的设置 |

新写的代码负责单页 UI、模型目录读取、客户端适配、备份事务和恢复。存储采用与上游相同的同目录临时文件原子替换策略，但使用 tempfile 实现，未复制上游 SQLite、代理和账户系统。

未包含上游的代理转发、自动故障切换、统计、MCP/Skills 管理、订阅登录、云同步、托盘、自动更新或深链接处理器。没有继承上游更新地址或应用标识，也不读取 CC Switch 数据库。


品牌资源：应用界面与打包图标使用 SuperGPT 的 S 标志，配色来自 [SuperGPT 控制台](https://console.supergpt.dev)。

VS Code 注释保留使用 `jsonc-parser` 0.34.0（MIT），许可证随包附带于 `licenses/jsonc-parser-MIT.txt`。

## Codex 模型能力资料

`src-tauri/src/resources/codex_model_profiles.json` 仅整理已知模型的 ID、默认档位、支持的档位名称、上下文容量、最大容量与相关百分比，来源为 OpenAI 发布的 Codex 模型目录：

- [rust-v0.114.0](https://github.com/openai/codex/blob/rust-v0.114.0/codex-rs/core/models.json)：保留早期 GPT-5 / Codex / GPT-OSS 模型。
- [rust-v0.143.0](https://github.com/openai/codex/blob/rust-v0.143.0/codex-rs/models-manager/models.json)：补充 GPT-5.4 Mini 等模型。
- [rust-v0.162.0](https://github.com/openai/codex/blob/rust-v0.162.0/codex-rs/models-manager/models.json)：补充 GPT-5.6 / GPT-6 系列；相同 ID 采用较新目录的档位。

这些资料用于模型目录只返回名称时的思考档位和上下文参数补全，不复制模型指令，不声明视觉能力。最大容量与默认容量分别保留，避免错误限制用户显式配置的窗口。网关明确返回的能力信息优先，包括空档位列表；未知别名不通过名称前缀猜测。`data` 和 `models` 两种目录格式均支持 Codex 原生能力字段。

v0.1.3 同时保留发布目录中的 `multi_agent_reasoning_effort`。Codex 0.162 将 Ultra 作为客户端模式，并通过该字段确定普通请求的档位；不能把菜单中的 Ultra 与发送到 Responses 的 `reasoning.effort` 当作永远相同的字符串。实现依据见 [OpenAI 的档位解析](https://github.com/openai/codex/blob/rust-v0.162.0/codex-rs/protocol/src/openai_models/reasoning_effort.rs)。旧版客户端可能采用自身的回退规则。


0.1.5 参考 CC Switch v4.0.15（988b0d00a45ab70aadc7aad683009269e6038f4d）CommonConfigEditor 的 ENABLE_TOOL_SEARCH 配置及 live/floor 的线路切换思路。Connect 另行实现按密钥与模型查询验证结果、导入前预览和本地冲突检查。Claude 桌面版采用原生 toolSearchEnabled；其 gateway 文档要求 1.21459.0 起支持，1.40609.0 起仅打开工具搜索相关 beta，故自动策略对已检测到的更早版本保守关闭。

参考：<https://code.claude.com/docs/en/mcp#configure-tool-search>；<https://claude.com/docs/third-party/claude-desktop/gateway#mcp-tool-search>。

## 0.1.6 全面对照

2026-10-19 再次核对 CC Switch v4.0.15：annotated tag `988b0d00a45ab70aadc7aad683009269e6038f4d`，实际源码 commit `79e754ab1b6ed50da57dae1d0faecab4b3b52f1c`；当日 main `adc223af322adaeca878e82bf78323865b0db768`。

- `src/utils/claudeEditorOverlay.ts`：第三方网关 AUTO_MODE_SERVER 默认关闭。
- `src/components/providers/forms/CommonConfigEditor.tsx`：Artifact 与 ToolSearch 独立开关；自动更新和提交署名属于用户偏好，Connect 不替用户改动。
- `src-tauri/src/live/project/codex.rs`：线路内 Bearer 凭据、保留官方登录、清理保留 provider 名称。Connect 使用独立 `supergpt-connect` ID，`requires_openai_auth=false` 避免回退到官方凭据；真实 0.143 / 0.162 请求验证。
- `src-tauri/src/live/floor.rs` / `claude_desktop_config.rs`：provider 字段归属、旧桌面网关迁移和 Windows 后缀目录；Connect 对多个候选目录报错，不任意选首个。
- main 中 5.5 proxy alias 修正用于 CC Switch 本地代理模型映射。Connect 直连保留网关返回的实际 ID，不移植无法反向映射的假别名。

具体分类、修复和未覆盖能力见 COMPATIBILITY.md。所有源自 CC Switch 的代码继续使用原 MIT 许可。
