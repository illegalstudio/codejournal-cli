import { test } from "node:test";
import assert from "node:assert/strict";
import { integration } from "../plugin.mjs";
import { payload } from "../tools.mjs";

const reply = (text) => ({ hookSpecificOutput: { additionalContext: text } });
function fixture() {
  const calls = [],
    toasts = [];
  const hooks = integration(
    { client: { tui: { showToast: async (body) => toasts.push(body) } } },
    async (event, session, detail) => {
      calls.push({ event, session, detail });
      if (event === "SessionStart") return reply(`Brief for ${session}, ${detail.source}`);
      if (event === "Stop") return { decision: "block", reason: "Log the work" };
      return {};
    },
  );
  return { calls, hooks, toasts };
}

test("session context is isolated, startup runs once and compaction reloads the brief", async () => {
  const { calls, hooks } = fixture();
  for (const id of ["one", "two"])
    await hooks.event({ event: { type: "session.created", properties: { info: { id } } } });
  for (const id of ["one", "two", "one"]) {
    const output = { system: [] };
    await hooks["experimental.chat.system.transform"]({ sessionID: id }, output);
    assert.ok(output.system.includes(`Brief for ${id}, startup`));
  }
  assert.equal(calls.filter((call) => call.event === "SessionStart").length, 2);
  await hooks.event({ event: { type: "session.compacted", properties: { sessionID: "one" } } });
  const output = { system: [] };
  await hooks["experimental.chat.system.transform"]({ sessionID: "one" }, output);
  assert.ok(output.system.includes("Brief for one, compact"));
});

test("prompt metadata and shell environment preserve attribution without recording user text", async () => {
  const { hooks, calls } = fixture();
  await hooks["chat.message"]({ sessionID: "one" }, { parts: [{ type: "text", text: "private prompt" }] });
  assert.deepEqual(
    calls.map((call) => call.event),
    ["SessionStart", "UserPromptSubmit"],
  );
  assert.ok(!JSON.stringify(calls).includes("private prompt"));
  const output = { env: { OTHER: "keep" } };
  await hooks["shell.env"]({ sessionID: "one" }, output);
  assert.deepEqual(output.env, { OTHER: "keep", CODE_JOURNAL_AGENT: "opencode", CJ_SESSION_ID: "one" });
});

test("tools carry only relevant file/commit metadata and preserve the tool result", async () => {
  const { hooks, calls } = fixture();
  const input = { sessionID: "one", tool: "write", args: { filePath: "src/app.rs", content: "secret body" } };
  await hooks["tool.execute.before"](input, { args: input.args });
  const output = { output: "Written", metadata: {} };
  await hooks["tool.execute.after"](input, output);
  assert.equal(calls.at(-1).detail.tool_name, "Write");
  assert.deepEqual(calls.at(-1).detail.tool_input, { file_path: "src/app.rs" });
  assert.ok(!JSON.stringify(calls).includes("secret body"));
  assert.ok(output.output.startsWith("Written"));
  assert.ok(output.output.includes("cj log add"));
  assert.deepEqual(payload("bash", { command: "git commit -m test" }, { exit: 0 }).tool_response, {
    exit_code: 0,
  });
  const patch = payload("apply_patch", {
    patchText: "*** Update File: a.rs\n-private\n+secret\n*** Move to: b.rs",
  });
  assert.equal(patch.tool_input.patch, "*** Update File: a.rs\n*** Move to: b.rs");
});

test("a conflict interrupts an edit once so the agent can review and retry", async () => {
  const hooks = integration({ client: {} }, async (event) =>
    event === "PreToolUse" ? reply("Check concurrent edit") : {},
  );
  const input = { sessionID: "one", tool: "write" },
    output = { args: { filePath: "a.rs" } };
  await assert.rejects(hooks["tool.execute.before"](input, output), /Check concurrent edit/);
  await hooks["tool.execute.before"](input, output);
  assert.deepEqual(output.args, { filePath: "a.rs" });
});

test("idle reminders do not start model calls and deleted sessions end", async () => {
  const { hooks, calls, toasts } = fixture();
  await hooks["chat.message"]({ sessionID: "one" }, { parts: [] });
  await hooks.event({
    event: { type: "permission.asked", properties: { sessionID: "one", permission: "bash" } },
  });
  await hooks.event({ event: { type: "session.idle", properties: { sessionID: "one" } } });
  assert.equal(toasts[0].body.message, "Log the work");
  await hooks.event({ event: { type: "session.deleted", properties: { info: { id: "one" } } } });
  assert.equal(calls.at(-1).event, "SessionEnd");
});

test("an unavailable journal adds actionable context without breaking generation", async () => {
  const hooks = integration({ client: {} }, async () => ({ unavailable: true }));
  const output = { system: ["Existing instructions"] };
  await hooks["experimental.chat.system.transform"]({ sessionID: "one" }, output);
  assert.equal(output.system[0], "Existing instructions");
  assert.ok(output.system.at(-1).includes("cj brief"));
});

test("sessions outside a project never receive work-log or compaction reminders", async () => {
  const hooks = integration({ client: {} }, async (event) =>
    event === "SessionStart" ? { ...reply("No project here"), journal_active: false } : {},
  );
  const system = { system: [] };
  await hooks["experimental.chat.system.transform"]({ sessionID: "outside" }, system);
  assert.deepEqual(system.system, ["No project here"]);
  const output = { output: "Written", metadata: {} };
  await hooks["tool.execute.after"]({ sessionID: "outside", tool: "write", args: {} }, output);
  assert.equal(output.output, "Written");
  const compact = { context: [] };
  await hooks["experimental.session.compacting"]({ sessionID: "outside" }, compact);
  assert.deepEqual(compact.context, []);
});

test("resumed sessions are identified and disabling hooks leaves agent output untouched", async () => {
  const { hooks, calls } = fixture();
  await hooks["experimental.chat.system.transform"]({ sessionID: "resumed" }, { system: [] });
  assert.equal(calls[0].detail.source, "resume");
  const prior = process.env.CODE_JOURNAL_HOOKS;
  try {
    process.env.CODE_JOURNAL_HOOKS = "off";
    const output = { output: "Written", metadata: {} };
    await hooks["tool.execute.after"]({ sessionID: "resumed", tool: "write", args: {} }, output);
    assert.equal(output.output, "Written");
    assert.equal(calls.length, 1);
  } finally {
    if (prior === undefined) delete process.env.CODE_JOURNAL_HOOKS;
    else process.env.CODE_JOURNAL_HOOKS = prior;
  }
});
