FAKE = r'''
import json, os, subprocess, sys, time
from pathlib import Path

if "CJ_DELEGATION_ID" not in os.environ:
    if sys.argv[1:] == ["provider","list","--json"]:
        print(json.dumps({"models":{"configured-model":{"display_name":"Configured model","provider":"synthetic-provider","model":"model-id"}},"providers":{"private_configuration":"must-not-be-exposed"}}))
        raise SystemExit(0)
    print("Synchronous fake provider response")
    raise SystemExit(0)
client = os.environ["CJ_DELEGATION_CLIENT"]
delegation_id = os.environ["CJ_DELEGATION_ID"]
attempt = int(os.environ["CJ_DELEGATION_ATTEMPT"])
behavior = os.environ.get("CJ_FAKE_BEHAVIOR", "finish")

def call(*args):
    result = subprocess.run([client, "delegation", *args], capture_output=True, text=True)
    if result.returncode:
        raise RuntimeError(result.stderr)
    return json.loads(result.stdout)

def publish(kind, body, *extra):
    return call("publish", delegation_id, "--role", "guest", "--kind", kind, "--body", body, *extra)

snapshot = call("sync", delegation_id, "--role", "guest", "--full")
publish("ready", "Protocol loaded")
if behavior == "noresult":
    print("I am analyzing; this is diagnostic, not a result", flush=True)
elif behavior == "streaming_review":
    streaming = '--output-format' in sys.argv and sys.argv[sys.argv.index('--output-format')+1] == 'streaming-json'
    for _ in range(10):
        if streaming and '--include-partial-messages' in sys.argv:
            print(json.dumps({'type':'tool_call', 'text':'Reading the next review input'}), flush=True)
        time.sleep(.4)
    publish("result", "Review completed with streamed activity")
elif behavior in ("sleep", "no_diff"):
    time.sleep(8)
elif behavior == "contention":
    deadline = time.monotonic()+60
    while not Path("release-guest").exists():
        if time.monotonic() > deadline:
            raise RuntimeError("Host did not release the contention fixture")
        time.sleep(.04)
    publish("result", "Completed after database contention")
elif behavior == "recover" and attempt == 1:
    publish("checkpoint", "Durable evidence from attempt one")
elif behavior == "recover":
    if not any(e["kind"] == "checkpoint" for e in snapshot["events"]):
        raise RuntimeError("Recovery lost its checkpoint")
    time.sleep(.3)
    publish("result", "Recovered from the durable checkpoint")
elif behavior == "write_without_review":
    Path("delegated.txt").write_text("delegated change\n")
    publish("result", "Unreviewed result must be rejected")
elif behavior in ("question", "write"):
    if behavior == "write":
        Path("delegated.txt").write_text("delegated change\n")
        publish("checkpoint", "delegated.txt; one insertion; no tests run")
    question = publish("question", "Please answer the bounded question or review the diff")
    after = question["event"]["seq"]
    deadline = time.monotonic()+10
    while time.monotonic() < deadline:
        response = call("wait", delegation_id, "--role", "guest", "--after", str(after), "--timeout", "0.2")
        if response["events"]:
            after = max(e["seq"] for e in response["events"])
        if any(e["kind"] == "answer" for e in response["events"]):
            publish("result", "Completed after the host answer")
            break
    else:
        raise RuntimeError("Host answer never arrived")
elif behavior == "impersonate":
    result = subprocess.run([client,"delegation","publish",delegation_id,"--role","host","--kind","message","--body","forged"], capture_output=True, text=True)
    if result.returncode == 0:
        raise RuntimeError("Guest impersonated the host")
    nested = subprocess.run([client,"delegation","create","claude","--prompt","nested"], capture_output=True, text=True)
    if nested.returncode == 0:
        raise RuntimeError("Nested delegation was allowed")
    publish("result", "Host impersonation and nested delegation rejected")
elif behavior == "hooks":
    environment = os.environ.copy()
    environment["CODE_JOURNAL_AGENT"] = "claude"
    result = subprocess.run([client,"hook","SessionStart"], input=json.dumps({"session_id":"synthetic-guest-session","cwd":str(Path.cwd())}), env=environment, capture_output=True, text=True)
    if result.returncode:
        raise RuntimeError(result.stderr)
    publish("result", "Guest hooks linked to the native delegation")
elif behavior == "mcp":
    environment = os.environ.copy()
    if Path(sys.argv[0]).name == "claude":
        config_path = sys.argv[sys.argv.index("--mcp-config")+1]
        config = json.loads(Path(config_path).read_text())["mcpServers"]["delegation"]
        if config["command"] != client or config["args"] != ["delegation","mcp"]:
            raise RuntimeError("Claude MCP is not owned by the installed binary")
        environment = {name:value for name,value in os.environ.items() if name in ("HOME","PATH")}
        environment.update(config.get("env",{}))
    if Path(sys.argv[0]).name == "codex":
        prefix = "mcp_servers.delegation.env_vars="
        forwarded = next((json.loads(arg[len(prefix):]) for arg in sys.argv if arg.startswith(prefix)), [])
        environment = {name:value for name,value in os.environ.items() if name in forwarded or name in ("HOME","PATH")}
    requests = [
        {"jsonrpc":"2.0","id":1,"method":"initialize","params":{}},
        {"jsonrpc":"2.0","id":2,"method":"tools/list"},
        {"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"delegation_sync","arguments":{"full":True}}},
        {"jsonrpc":"2.0","id":4,"method":"tools/call","params":{"name":"delegation_finish","arguments":{"body":"Completed through embedded MCP"}}},
    ]
    result = subprocess.run([client,"delegation","mcp"], input="".join(json.dumps(r)+"\n" for r in requests), env=environment, capture_output=True, text=True, check=True)
    responses = [json.loads(line) for line in result.stdout.splitlines()]
    if any("error" in response for response in responses):
        raise RuntimeError(result.stdout)
    if len(responses[1]["result"]["tools"]) != 12:
        raise RuntimeError("Missing delegation MCP tools")
else:
    publish("result", "Standalone provider completed")
'''
