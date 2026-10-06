export function payload(tool, args = {}, metadata = {}) {
  args = args && typeof args === "object" ? args : {};
  const names = { bash: "Bash", edit: "Edit", write: "Write", apply_patch: "apply_patch" };
  const input = {};
  if (tool === "bash" && typeof args.command === "string") input.command = args.command;
  if (["edit", "write"].includes(tool) && typeof args.filePath === "string") {
    input.file_path = args.filePath;
  }
  if (tool === "apply_patch" && typeof args.patchText === "string") {
    input.patch = args.patchText
      .split("\n")
      .filter((line) => /^\*\*\* (?:Add|Update|Delete) File: |^\*\*\* Move to: /.test(line))
      .join("\n");
  }
  return {
    tool_name: names[tool] ?? tool,
    tool_input: input,
    tool_response: Number.isInteger(metadata?.exit) ? { exit_code: metadata.exit } : {},
  };
}

export function changed(tool, args, metadata) {
  return (
    ["edit", "write", "apply_patch"].includes(tool) ||
    (tool === "bash" && metadata?.exit === 0 && /\bgit\b[^\n]*\bcommit\b/.test(args?.command ?? ""))
  );
}
