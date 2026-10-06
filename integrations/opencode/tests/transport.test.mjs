import { test } from "node:test";
import assert from "node:assert/strict";
import { mkdtemp, writeFile, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { transport } from "../transport.mjs";
import { sessions } from "../sessions.mjs";

test("the child receives structured stdin and attribution, and invalid/missing executables fail open", async () => {
  const directory = await mkdtemp(join(tmpdir(), "cj-opencode-transport-"));
  try {
    await writeFile(
      join(directory, "hook"),
      `
      let input = '';
      process.stdin.on('data', chunk => input += chunk);
      process.stdin.on('end', () => process.stdout.write(JSON.stringify({
        event: process.argv[2], payload: JSON.parse(input),
        agent: process.env.CODE_JOURNAL_AGENT, session: process.env.CJ_SESSION_ID
      })));
    `,
    );
    const run = transport(process.execPath, directory);
    const result = await run("SessionStart", "session ' $()", { source: "startup" });
    assert.equal(result.event, "SessionStart");
    assert.equal(result.agent, "opencode");
    assert.equal(result.session, "session ' $()");
    assert.equal(result.payload.cwd, directory);
    await writeFile(join(directory, "hook"), "process.stdout.write('not json')");
    assert.deepEqual(await run("Stop", "one"), { unavailable: true });
    await writeFile(join(directory, "hook"), "process.stdout.write('null')");
    assert.deepEqual(await run("Stop", "one"), { unavailable: true });
    assert.deepEqual(await transport(join(directory, "missing"), directory)("Stop", "one"), {
      unavailable: true,
    });
    await writeFile(join(directory, "hook"), "setInterval(() => {}, 1000)");
    const start = Date.now();
    assert.deepEqual(await run("Stop", "one"), { unavailable: true });
    assert.ok(Date.now() - start < 8000);
  } finally {
    await rm(directory, { recursive: true, force: true });
  }
});

test("events in one session serialize, and other sessions proceed independently", async () => {
  const completed = [];
  let release;
  const wait = new Promise((resolve) => {
    release = resolve;
  });
  const journal = sessions(async (event, id) => {
    if (event === "first") await wait;
    completed.push(`${id}:${event}`);
    return {};
  }, {});
  const first = journal.serial("one", "first", {});
  const second = journal.serial("one", "second", {});
  await journal.serial("two", "other", {});
  assert.deepEqual(completed, ["two:other"]);
  release();
  await Promise.all([first, second]);
  assert.deepEqual(completed, ["two:other", "one:first", "one:second"]);
});
