# Connect / CC Switch 兼容性对照

复核日期：2026-10-19。基线是 CC Switch v4.0.15 的实际源码 `79e754ab` 和 main `adc223af`，并用真实 Claude Code 2.1.292、Codex 0.143.0 / 0.162.0 复核客户端行为。这里记录配置器能力，不把成功读取模型目录当作全功能认证。

| 检查项 | Connect 0.1.6 的处理 | 验证 |
| --- | --- | --- |
| MCP 按需加载 | 保留按线路三态能力；全选模型支持才自动开启，未知提示，已知不支持不能强开 | 100 工具、搜索 → tool_reference → 定义 → 调用结果；权限与混合模型回归 |
| Claude 自动模式 | 与 CC Switch 第三方默认一致，关闭服务端分类检查；不改审批模式 | 实际客户端接入、配置与权限保留断言 |
| Claude 用途模型 | 唯一同系列已选模型自动对应；允许自选 Haiku/Sonnet/Opus/Fable；子代理默认继承 | 高价主模型与轻量 Haiku 分开，真实 CLI `--model haiku` 请求验证 |
| Artifact 格式 | 独立保留/禁用/允许开关，保留既有值 | 与 MCP 设置独立，CLI / 扩展使用同一生成规则 |
| 旧环境变量 | 不只删文件字段；显式清空继承的旧 API Key、OAuth、模型锁定、云平台开关 | 真实 Claude 继承错误 Key / 模型 / Bedrock / Vertex 后仍用正确线路 |
| VS Code 登录与目录 | 关闭官方登录提示；读取扩展绝对 CLAUDE_CONFIG_DIR，禁止相对/重复值 | 写入、预览、恢复指向同一目录，默认 CLI 文件保持 |
| 版本与覆盖 | ToolSearch 低于 2.1.14 提示；模型菜单低于 2.1.242 提示；桌面按需加载需 1.40609.0+ | 语义版本比较、未知版本提示；项目/管理策略仍可覆盖 |
| Codex 鉴权 | 服务商表中保存 Bearer Key，保留 auth.json 与 keyring 偏好 | 无登录、原 API Key、原 ChatGPT 登录、继承错误环境 Key，实际请求认证 |
| Codex 保留名称 | 备份并移除旧 openai/ollama/lmstudio provider 表；其他 provider 保留 | 真实客户端正常加载、精确恢复 |
| Codex 联网搜索 | 保留现有值或选择关闭/缓存/实时；新安装默认关闭并明确显示 | 真实 Responses 请求搜索工具声明；未宣称所有线路能执行搜索 |
| 模型目录与容量 | 保留图片元数据、按模型思考档位、真实容量、手工容量设置；未知容量阻止写入 | 原全量目录、图片 URL/本地 PNG、数字/Ultra 思考、压缩阈值回归 |
| 桌面旧配置 | 独立 profile；迁移本地 enterpriseConfig 网关字段，保留策略/MCP | 四文件事务、重复 profile ID 拒绝、精确恢复 |
| Windows 路径 | 精确目录优先，唯一有配置的后缀目录可识别，多个候选报错 | 跨平台目录选择用例、Windows 原生 CI |
| 文件保护 | 首次备份、文件锁、原子替换、中断恢复；编辑后恢复前核对且留安全副本 | 中文空格路径、CRLF/BOM/JSONC/TOML 注释、坏文件、并发/过期恢复预览 |
| 不随线路改变的偏好 | 保留自动更新、提交署名、MCP/插件/hooks、审批权限；保留容量/思考偏好并提示 | 回归比较原字段和恢复后的原始字节 |

## 适用边界

Connect 是本地直连配置器。CC Switch 的本地代理、跨协议转换、故障转移、OAuth 供应商登录、用量面板、MCP/Skills 管理与 Gemini/OpenCode 等其他客户端属于不同产品功能。Connect 不会伪造这些能力。只支持提供 Bearer 模型目录的站点；第三方协议和工具能力取决于实际线路。

没有自动更改组织策略、VS Code 自定义 Profile、Remote SSH / WSL 或项目内配置。无法读取的远端/系统策略和启动脚本可能覆盖本机设置，界面会提醒重启并检查。Windows 桌面版本不能可靠自动读取时会提示最低版本。测试使用虚构 Key 和本机模拟网关，不产生真实推理费用；它验证客户端发送能力，不替代维护者对实际线路的验证。

服务端按需加载证据继续使用 0.1.5 发布时逐条线路实测及配置指纹/有效期；本次未扩大服务端能力声明、未修改价格或账目。联网搜索及 Artifact 是明确的兼容选项，不通过关闭所有实验功能规避问题。

## 主要依据与持续回归

- [CC Switch 发布源码](https://github.com/farion1231/cc-switch/tree/v4.0.15)、[当前审查 main](https://github.com/farion1231/cc-switch/tree/adc223af322adaeca878e82bf78323865b0db768)
- [Claude 环境变量及空值覆盖](https://code.claude.com/docs/en/env-vars)、[模型用途](https://code.claude.com/docs/en/model-config)、[VS Code 第三方设置](https://code.claude.com/docs/en/vs-code)、[配置优先级](https://code.claude.com/docs/en/settings)
- [Codex 自定义服务商](https://learn.chatgpt.com/docs/config-file/config-advanced#custom-model-providers)

每次提交的原生 CI 跑全部 Rust 配置/恢复测试、前端选择状态测试、真实 Codex 16 场景以及两平台发布包。`scripts/verify-claude-runtime.py` 在安装 Claude Code 的环境运行，支持 default、inherited、role 场景。具体运行版本、发布产物与界面范围见 VERIFICATION.md。
