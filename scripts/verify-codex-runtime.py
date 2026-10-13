"""Exercise real Codex against Connect's production importer on macOS/Windows.

No real credentials, paid requests, or writes to the user's client configuration.
Usage: cargo build --manifest-path src-tauri/Cargo.toml --no-default-features --example codex_fixture
       python scripts/verify-codex-runtime.py --download-codex
"""
import argparse
import gzip
import hashlib
import json
import os
from pathlib import Path
import platform
import queue
import shutil
import subprocess
import tarfile
import tempfile
import threading
import time
import urllib.request
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer

ROOT = Path(__file__).resolve().parents[1]
VERSION = "0.162.0"
FAKE_KEY = "sk-connect-fixture-only"
PROFILES = json.loads((ROOT / "src-tauri/src/resources/codex_model_profiles.json").read_text())


def download_codex(directory):
    system, machine = platform.system(), platform.machine().lower()
    if system == "Windows":
        name = "codex-x86_64-pc-windows-msvc.exe"
    elif system == "Darwin":
        arch = "aarch64" if machine in ("arm64", "aarch64") else "x86_64"
        name = f"codex-{arch}-apple-darwin.tar.gz"
    else:
        raise RuntimeError("Use --codex PATH outside macOS and Windows")
    headers = {"User-Agent": "SuperGPT-Connect-verification"}
    if token := os.environ.get("GITHUB_TOKEN"):
        headers["Authorization"] = "Bearer " + token
    req = urllib.request.Request(
        f"https://api.github.com/repos/openai/codex/releases/tags/rust-v{VERSION}",
        headers=headers,
    )
    with urllib.request.urlopen(req, timeout=30) as response:
        release = json.load(response)
    asset = next(a for a in release["assets"] if a["name"] == name)
    assert asset["browser_download_url"].startswith("https://github.com/openai/codex/releases/download/")
    archive = directory / name
    urllib.request.urlretrieve(asset["browser_download_url"], archive)
    assert "sha256:" + hashlib.sha256(archive.read_bytes()).hexdigest() == asset["digest"]
    if system == "Windows":
        return archive
    with tarfile.open(archive) as bundle:
        member = next(m for m in bundle.getmembers() if m.isfile() and Path(m.name).name == name.removesuffix(".tar.gz"))
        binary = directory / "codex"
        with bundle.extractfile(member) as source, binary.open("wb") as destination:
            shutil.copyfileobj(source, destination)
    binary.chmod(0o700)
    return binary


class Gateway(BaseHTTPRequestHandler):
    def log_message(self, *args):
        pass

    def do_GET(self):
        assert self.path == "/v1/models", self.path
        assert self.headers.get("Authorization") == "Bearer " + FAKE_KEY
        body = json.dumps(self.server.payload).encode()
        self.send_response(200)
        self.send_header("Content-Type", "application/json")
        self.send_header("Content-Length", str(len(body)))
        self.end_headers()
        self.wfile.write(body)

    def do_POST(self):
        assert self.path == "/v1/responses", self.path
        assert self.headers.get("Authorization") == "Bearer " + FAKE_KEY
        body = self.rfile.read(int(self.headers["Content-Length"]))
        if self.headers.get("Content-Encoding") == "gzip":
            body = gzip.decompress(body)
        data = json.loads(body)
        normal = "CTX_FIXTURE_" in json.dumps(data.get("input", [])[-1:])
        tokens = self.server.usage if normal else 100
        self.server.requests.append({"model": data.get("model"), "reasoning": data.get("reasoning"),
                                     "normal_turn": normal, "reported_input_tokens": tokens})
        message = {"id": "msg_fixture", "type": "message", "status": "completed", "role": "assistant",
                   "content": [{"type": "output_text", "text": "OK", "annotations": []}]}
        response = {"id": "resp_" + str(len(self.server.requests)), "object": "response", "model": data["model"],
                    "status": "completed", "output": [message],
                    "usage": {"input_tokens": tokens, "output_tokens": 1, "total_tokens": tokens + 1}}
        stream = [
            {"type": "response.created", "response": {**response, "status": "in_progress", "output": []}},
            {"type": "response.output_item.added", "output_index": 0, "item": {**message, "status": "in_progress", "content": []}},
            {"type": "response.output_text.delta", "item_id": "msg_fixture", "output_index": 0, "content_index": 0, "delta": "OK"},
            {"type": "response.output_item.done", "output_index": 0, "item": message},
            {"type": "response.completed", "response": response},
        ]
        encoded = "".join("event: " + e["type"] + "\ndata: " + json.dumps(e) + "\n\n" for e in stream).encode()
        self.send_response(200)
        self.send_header("Content-Type", "text/event-stream")
        self.send_header("Content-Length", str(len(encoded)))
        self.end_headers()
        self.wfile.write(encoded)


class Codex:
    def __init__(self, executable, home):
        env = os.environ.copy()
        for key in ("OPENAI_API_KEY", "OPENAI_BASE_URL", "CODEX_PROFILE"):
            env.pop(key, None)
        env["CODEX_HOME"] = str(home / ".codex")
        self.process = subprocess.Popen([str(executable), "app-server", "--stdio"], cwd=home, env=env,
                                        stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.PIPE,
                                        text=True, encoding="utf-8", errors="replace")
        self.events, self.errors, self.incoming, self.sequence = [], [], queue.Queue(), 0
        def output():
            for line in self.process.stdout:
                try:
                    self.incoming.put(json.loads(line))
                except json.JSONDecodeError:
                    pass
        def errors():
            self.errors.extend(self.process.stderr)
        threading.Thread(target=output, daemon=True).start()
        threading.Thread(target=errors, daemon=True).start()
        self.initialized = self.rpc("initialize", {"clientInfo": {"name": "connect_regression", "version": "1"},
                                                  "capabilities": {"experimentalApi": True}})
        self.send({"method": "initialized"})

    def send(self, message):
        self.process.stdin.write(json.dumps(message) + "\n")
        self.process.stdin.flush()

    def receive(self, predicate):
        deadline = time.monotonic() + 35
        while time.monotonic() < deadline:
            try:
                message = self.incoming.get(timeout=max(.1, deadline - time.monotonic()))
            except queue.Empty:
                raise RuntimeError("Codex timed out: " + "".join(self.errors[-8:]))
            self.events.append(message)
            if predicate(message):
                return message
        raise TimeoutError("Codex response timed out")

    def rpc(self, method, params):
        self.sequence += 1
        self.send({"id": self.sequence, "method": method, "params": params})
        reply = self.receive(lambda m: m.get("id") == self.sequence)
        assert "error" not in reply, reply
        return reply["result"]

    def start(self, home, model):
        reply = self.rpc("thread/start", {"cwd": str(home), "model": model, "approvalPolicy": "never",
                                          "sandbox": "read-only", "ephemeral": True})
        self.last_start = reply
        return reply["thread"]["id"]

    def turn(self, thread, model, effort=None):
        params = {"threadId": thread, "model": model,
                  "input": [{"type": "text", "text": "CTX_FIXTURE_: Reply OK without tools."}]}
        if effort is not None:
            params["effort"] = effort
        self.rpc("turn/start", params)
        reply = self.receive(lambda m: m.get("method") == "turn/completed" and m["params"]["threadId"] == thread)
        assert reply["params"]["turn"]["status"] == "completed", reply
        usage = [m["params"]["tokenUsage"] for m in self.events
                 if m.get("method") == "thread/tokenUsage/updated" and m["params"]["threadId"] == thread]
        assert usage, "Missing actual context-window report"
        return usage[-1]["modelContextWindow"]

    def close(self):
        self.process.terminate()
        try:
            self.process.wait(timeout=5)
        except subprocess.TimeoutExpired:
            self.process.kill()
            self.process.wait()


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--codex", type=Path)
    parser.add_argument("--download-codex", action="store_true")
    parser.add_argument("--output", type=Path, default=ROOT / "test-results/codex-runtime")
    args = parser.parse_args()
    args.output.mkdir(parents=True, exist_ok=True)
    fixture = ROOT / "src-tauri/target/debug/examples" / ("codex_fixture.exe" if os.name == "nt" else "codex_fixture")
    assert fixture.is_file(), "Build the codex_fixture example first"
    report = {"platform": platform.platform(), "real_provider_calls": 0, "real_user_configuration_changed": False, "cases": []}
    with tempfile.TemporaryDirectory(prefix="connect-runtime-") as temporary:
        temp = Path(temporary).resolve()
        binary = args.codex.resolve() if args.codex else download_codex(temp)
        report["codex_version"] = subprocess.check_output([str(binary), "--version"], text=True).strip()
        gateway = ThreadingHTTPServer(("127.0.0.1", 0), Gateway)
        gateway.requests, gateway.usage = [], 100
        threading.Thread(target=gateway.serve_forever, daemon=True).start()
        endpoint = "http://127.0.0.1:" + str(gateway.server_port)

        def run_case(label, payload, models, config="", usage=None, all_efforts=False, legacy=False, repair=False):
            home = temp / (label + " 中文 User")
            home.mkdir()
            (home / ".connect-runtime-fixture").touch()
            (home / ".codex").mkdir()
            baseline = ("# Original user configuration\r\n" + config).encode()
            original_auth = b'{\r\n  "OPENAI_API_KEY": "original-fake-key"\r\n}\r\n'
            (home / ".codex/config.toml").write_bytes(baseline)
            (home / ".codex/auth.json").write_bytes(original_auth)
            spec = {"home": str(home), "root": endpoint, "client": "codex-desktop" if len(report["cases"]) % 2 == 0 else "codex",
                    "selectedModels": models, "defaultModel": models[0]}
            request = temp / (label + ".json")
            def apply(restore=False):
                request.write_text(json.dumps({**spec, "restore": restore}), encoding="utf-8")
                result = subprocess.run([str(fixture), str(request)], capture_output=True, text=True, encoding="utf-8")
                assert result.returncode == 0, result.stderr
                return json.loads(result.stdout)
            gateway.payload, gateway.requests, gateway.usage = payload, [], 100
            apply()
            catalog_path = home / ".codex/supergpt-models.json"
            if legacy:
                old = json.loads(catalog_path.read_text())
                old.pop("_supergpt_connect", None)
                for row in old["models"]:
                    row.update(context_window=32768, max_context_window=32768)
                catalog_path.write_text(json.dumps(old))
                apply()
            if repair:
                edited = json.loads(catalog_path.read_text())
                edited["models"][0]["context_window"] = 500000
                edited["models"][0]["auto_compact_token_limit"] = 440000
                catalog_path.write_text(json.dumps(edited))
                gateway.payload = {"data": [{"id": models[0], "effective_context_window_percent": 90}]}
                apply()
            assert not apply()["changed"], "Reimport must be stable"
            catalog = json.loads(catalog_path.read_text())
            if legacy:
                assert catalog["models"][0]["max_context_window"] == 872000
            if repair:
                assert catalog["models"][0]["context_window"] == 500000
                assert catalog["models"][0]["effective_context_window_percent"] == 90
            client = Codex(binary, home)
            case = {"name": label, "client": spec["client"], "models": [], "passed": False}
            report["cases"].append(case)
            try:
                listed = client.rpc("model/list", {"includeHidden": True, "limit": 100})["data"]
                assert {m["model"] for m in listed} == set(models)
                for row in catalog["models"]:
                    model = row["slug"]
                    picked = next(m for m in listed if m["model"] == model)
                    efforts = [v["effort"] for v in row["supported_reasoning_levels"]]
                    assert [v["reasoningEffort"] for v in picked["supportedReasoningEfforts"]] == efforts
                    default = row["default_reasoning_level"]
                    if default is not None:
                        assert picked["defaultReasoningEffort"] == default
                    window = row["context_window"] or row["max_context_window"]
                    if "model_context_window = 500000" in config:
                        window = min(500000, row["max_context_window"] or 500000)
                    expected = window * row["effective_context_window_percent"] // 100
                    levels = efforts if all_efforts and efforts else [default]
                    observed = []
                    for effort in levels:
                        thread = client.start(home, model)
                        if 'model_reasoning_effort = "ultra"' in config:
                            assert client.last_start["reasoningEffort"] == ("ultra" if "ultra" in efforts else None)
                        before = len(gateway.requests)
                        actual = client.turn(thread, model, effort)
                        assert actual == expected, (model, actual, expected)
                        sent = gateway.requests[before:]
                        assert len(sent) == 1 and sent[0]["model"] == model, sent
                        if row["supports_reasoning_summaries"]:
                            # Codex 0.162 treats Ultra as a local mode, resolving its ordinary
                            # request to the model's multi-agent effort (or Max). This is the
                            # official protocol's resolve_reasoning_effort, not a lost selection.
                            expected_effort = effort
                            if effort == "ultra":
                                mapped = row.get("multi_agent_reasoning_effort") if "0.162." in report["codex_version"] else None
                                expected_effort = mapped or (
                                    "max" if "max" in efforts else next(e for e in reversed(efforts) if e != "ultra"))
                            if isinstance(expected_effort, str) and expected_effort.isdecimal() and "0.162." in report["codex_version"]:
                                expected_effort = int(expected_effort)
                            assert sent[0]["reasoning"]["effort"] == expected_effort, (model, effort, sent)
                        else:
                            assert not sent[0]["reasoning"], sent
                        observed.append(effort)
                    case["models"].append({"id": model, "effective_context": expected, "efforts_sent": observed})
                if usage is not None:
                    model = models[0]
                    thread = client.start(home, model)
                    gateway.usage = usage
                    client.turn(thread, model, "low")
                    gateway.usage = 100
                    before_events = len(client.events)
                    client.turn(thread, model, "low")
                    compact = [m for m in client.events[before_events:] if m.get("method") == "item/started"
                               and m.get("params", {}).get("item", {}).get("type") == "contextCompaction"]
                    assert len(compact) == (1 if usage > 450000 else 0), (usage, len(compact))
                    case.update(reported_usage=usage, compaction_events=len(compact))
                case["passed"] = True
            finally:
                client.close()
                (args.output / (label + "-events.json")).write_text(json.dumps(client.events, indent=2), encoding="utf-8")
                (args.output / (label + "-stderr.log")).write_text("".join(client.errors), encoding="utf-8")
                case["requests"] = gateway.requests
            apply(restore=True)
            assert (home / ".codex/config.toml").read_bytes() == baseline
            assert (home / ".codex/auth.json").read_bytes() == original_auth
            assert not catalog_path.exists()
            case["restore_exact"] = True
            print("PASS", label, flush=True)

        try:
            all_models = list(PROFILES)
            run_case("known-data", {"data": [{"id": model} for model in all_models]}, all_models, all_efforts=True)
            run_case("native-models", {"models": [{"slug": "gpt-6.1-sol"}]}, ["gpt-6.1-sol"], all_efforts=True)
            config = 'model_context_window = 500000\r\nmodel_auto_compact_token_limit = 450000\r\nmodel_reasoning_effort = "ultra"\r\n'
            for label, tokens in [("legacy-upgrade", 40000), ("below-threshold", 449000), ("above-threshold", 451000)]:
                run_case(label, {"data": [{"id": "gpt-6.1-sol"}]}, ["gpt-6.1-sol"], config,
                         usage=tokens, legacy=label == "legacy-upgrade")
            run_case("partial-metadata-repair", {"data": [{"id": "gpt-6.1-sol"}]}, ["gpt-6.1-sol"], repair=True)
            run_case("gateway-limits", {"data": [{"id": "gpt-6.1-sol", "context_window": 65536,
                                                   "supported_reasoning_levels": [{"effort": "high"}]}]}, ["gpt-6.1-sol"], config)
            run_case("gateway-no-reasoning", {"data": [{"id": "gpt-6.1-sol", "context_window": 65536,
                                                         "supported_reasoning_levels": []}]}, ["gpt-6.1-sol"], config)
            run_case("gateway-alias", {"data": [{"id": "custom-model", "context_window": 131072,
                                                  "supported_reasoning_levels": [{"effort": "low"}, {"effort": "high"}],
                                                  "default_reasoning_level": "high"}]}, ["custom-model"], all_efforts=True)
            run_case("gateway-numeric-efforts", {"data": [{"id": "numeric-model", "context_window": 131072,
                    "supported_reasoning_levels": [{"effort": 1024}, {"effort": 4096}],
                    "default_reasoning_level": 4096}]}, ["numeric-model"], all_efforts=True)
            report["passed"] = True
        finally:
            gateway.shutdown()
            gateway.server_close()
            (args.output / "summary.json").write_text(json.dumps(report, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    print(json.dumps({"passed": True, "cases": len(report["cases"]), "codex": report["codex_version"]}))


if __name__ == "__main__":
    main()
