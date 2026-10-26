import { transport, context } from "./transport.mjs";
import { sessions } from "./sessions.mjs";
import { payload, changed } from "./tools.mjs";

export function integration({ client }, invoke) {
  const journal = sessions(invoke, client);
  const hooks = {
    event: journal.event,
    "chat.message": async (input, output) => {
      if (output.parts?.length && output.parts.every((part) => part.synthetic)) return;
      await journal.start(input.sessionID);
      await journal.serial(input.sessionID, "UserPromptSubmit", {});
      journal.state(input.sessionID).conflicts.clear();
    },
    "experimental.chat.system.transform": async ({ sessionID }, output) => {
      if (!sessionID || process.env.CODE_JOURNAL_HOOKS === "off") return;
      const brief = await journal.start(sessionID);
      if (journal.state(sessionID).active)
        output.system.push(
          "Use the code-journal skill. Record durable work with cj log add before finishing a turn.",
        );
      if (brief) output.system.push(brief);
      output.system.push(...journal.state(sessionID).notices.splice(0));
    },
    "shell.env": async ({ sessionID }, output) => {
      if (!sessionID) return;
      output.env.CODE_JOURNAL_AGENT = "opencode";
      output.env.CJ_SESSION_ID = sessionID;
    },
    "tool.execute.before": async (input, output) => {
      await journal.start(input.sessionID);
      if (input.tool === "question")
        await journal.serial(input.sessionID, "Notification", {
          notification_type: "agent_needs_input",
          message: "OpenCode needs your input",
        });
      const notice = context(
        await journal.serial(input.sessionID, "PreToolUse", payload(input.tool, output.args)),
      );
      const warnings = journal.state(input.sessionID).conflicts;
      if (notice && !warnings.has(notice)) {
        warnings.add(notice);
        throw new Error(notice);
      }
    },
    "tool.execute.after": async (input, output) => {
      await journal.serial(input.sessionID, "PostToolUse", payload(input.tool, input.args, output.metadata));
      if (journal.state(input.sessionID).active && changed(input.tool, input.args, output.metadata)) {
        output.output +=
          "\n\nCode Journal: record this turn's durable work once with cj log add before finishing.";
      }
    },
    "experimental.session.compacting": async ({ sessionID }, output) => {
      await journal.start(sessionID);
      await journal.serial(sessionID, "PreCompact", {});
      if (journal.state(sessionID).active)
        output.context.push(
          "Preserve unresolved Code Journal work and record durable discoveries with cj add. The journal brief will reload after compaction.",
        );
    },
  };
  return Object.fromEntries(
    Object.entries(hooks).map(([name, hook]) => [
      name,
      async (...args) => {
        if (process.env.CODE_JOURNAL_HOOKS === "off") return;
        return hook(...args);
      },
    ]),
  );
}

export function createPlugin(binary) {
  return async (ctx) => integration(ctx, transport(binary, ctx.directory));
}
