import { context } from "./transport.mjs";

export function sessions(invoke, client) {
  const states = new Map();
  function state(id) {
    if (!states.has(id))
      states.set(id, {
        queue: Promise.resolve(),
        brief: null,
        source: "resume",
        notices: [],
        conflicts: new Set(),
        active: true,
      });
    return states.get(id);
  }
  function serial(id, event, detail) {
    const value = state(id);
    const next = value.queue.then(() => invoke(event, id, detail));
    value.queue = next.catch(() => ({}));
    return next;
  }
  async function start(id) {
    const value = state(id);
    if (!value.brief) {
      value.brief = serial(id, "SessionStart", { source: value.source }).then((result) => {
        value.active = result.journal_active !== false;
        if (result.unavailable)
          return "Code Journal is unavailable. Run cj brief before relying on journal context.";
        return context(result);
      });
    }
    return value.brief;
  }
  async function event({ event }) {
    const props = event.properties ?? {};
    const id = props.sessionID ?? props.info?.id;
    if (!id) return;
    if (event.type === "session.created") {
      state(id).source = "startup";
      return;
    }
    // Do not create session state merely by listing or deleting an unopened session.
    if (!states.has(id)) return;
    if (event.type === "session.idle") {
      const notice = context(await serial(id, "Stop", {}));
      if (notice) {
        state(id).notices.push(notice);
        await client.tui
          ?.showToast?.({ body: { title: "Code Journal", message: notice, variant: "warning" } })
          .catch(() => {});
      }
    }
    if (event.type === "permission.asked")
      await serial(id, "PermissionRequest", { tool_name: props.permission ?? "a tool" });
    if (event.type === "session.compacted") {
      state(id).source = "compact";
      state(id).brief = null;
    }
    if (event.type === "session.deleted") {
      await serial(id, "SessionEnd", { reason: "deleted" });
      states.delete(id);
    }
  }
  return { state, serial, start, event };
}
