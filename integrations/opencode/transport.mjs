import { execFile } from "node:child_process";

export function transport(binary, directory) {
  return (event, session, detail = {}) =>
    new Promise((resolve) => {
      if (process.env.CODE_JOURNAL_HOOKS === "off") return resolve({});
      const child = execFile(
        binary,
        ["hook", event],
        {
          cwd: directory,
          env: { ...process.env, CODE_JOURNAL_AGENT: "opencode", CJ_SESSION_ID: session },
          timeout: event === "SessionStart" ? 12000 : 5000,
          maxBuffer: 1024 * 1024,
          windowsHide: true,
        },
        (error, stdout) => {
          if (error) return resolve({ unavailable: true });
          try {
            const result = stdout.trim() ? JSON.parse(stdout) : {};
            resolve(
              result && typeof result === "object" && !Array.isArray(result) ? result : { unavailable: true },
            );
          } catch {
            resolve({ unavailable: true });
          }
        },
      );
      child.stdin.on("error", () => {});
      child.stdin.end(JSON.stringify({ ...detail, session_id: session, cwd: directory }));
    });
}

export function context(result) {
  const value =
    result?.hookSpecificOutput?.additionalContext ?? (result?.decision === "block" ? result.reason : "");
  return typeof value === "string" ? value : "";
}
