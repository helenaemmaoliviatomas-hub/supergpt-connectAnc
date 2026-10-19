import type { Client, SetupOptions } from "./native";

export default function SetupOptionsPanel({ client, models, value, change, disabled }: {
  client: Client; models: string[]; value: SetupOptions;
  change: (value: SetupOptions) => void; disabled: boolean;
}) {
  if (client.startsWith("codex")) return <label className="capability-setting">
    <span>联网搜索</span>
    <select aria-label="联网搜索" value={value.webSearch} disabled={disabled}
      onChange={event => change({ ...value, webSearch: event.target.value as SetupOptions["webSearch"] })}>
      <option value="preserve">保留现有设置（首次默认关闭）</option>
      <option value="disabled">关闭</option><option value="cached">缓存搜索</option><option value="live">实时搜索</option>
    </select>
  </label>;
  if (client === "claude-desktop") return null;
  const roles = [["haiku", "轻量后台任务 / Haiku"], ["sonnet", "Sonnet 别名"], ["opus", "Opus 别名"],
    ["fable", "Fable 别名"], ["subagent", "子代理模型"]] as const;
  return <details className="setup-options">
    <summary>高级设置：用途模型与兼容性</summary>
    <p>自动优先使用唯一勾选的同系列模型；没有明确匹配时使用默认模型。子代理默认遵循自身配置和当前会话。</p>
    {roles.map(([role, label]) => <label className="capability-setting" key={role}>
      <span>{label}</span>
      <select aria-label={label} value={value.claudeModels[role] ?? ""} disabled={disabled}
        onChange={event => {
          const next = { ...value.claudeModels };
          if (event.target.value) next[role] = event.target.value; else delete next[role];
          change({ ...value, claudeModels: next });
        }}>
        <option value="">{role === "subagent" ? "遵循子代理 / 当前会话" : "自动匹配已选模型"}</option>
        {models.map(model => <option key={model} value={model}>{model}</option>)}
      </select>
    </label>)}
    <label className="capability-setting"><span>Artifact 工具</span>
      <select aria-label="Artifact 工具" value={value.artifactMode} disabled={disabled}
        onChange={event => change({ ...value, artifactMode: event.target.value as SetupOptions["artifactMode"] })}>
        <option value="preserve">保留现有设置</option><option value="disabled">禁用（严格工具格式兼容）</option><option value="enabled">允许</option>
      </select>
    </label>
    <p>只有线路拒绝 Artifact 工具格式时才需要禁用；此开关不会关闭 MCP 按需加载。</p>
  </details>;
}
