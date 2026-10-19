import test from "node:test";
import assert from "node:assert/strict";
import { DEFAULT_OPTIONS, selectedOptions } from "../src/setup-options.ts";

test("unchecking a role's model removes its hidden override without changing another role", () => {
  const value = { ...DEFAULT_OPTIONS, claudeModels: { haiku: "light", subagent: "main" } };
  assert.deepEqual(selectedOptions("claude", ["main"], value).claudeModels, { subagent: "main" });
  assert.deepEqual(value.claudeModels, { haiku: "light", subagent: "main" });
});
test("client-specific settings cannot leak between desktop, Code and Codex", () => {
  const value = { claudeModels: { haiku: "light" }, artifactMode: "disabled" as const, webSearch: "live" as const };
  assert.deepEqual(selectedOptions("claude-desktop", ["light"], value), DEFAULT_OPTIONS);
  assert.deepEqual(selectedOptions("codex", ["light"], value), { ...DEFAULT_OPTIONS, webSearch: "live" });
  assert.equal(selectedOptions("claude-vscode", ["light"], value).webSearch, "preserve");
});
