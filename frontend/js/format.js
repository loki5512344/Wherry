// Formatting + TaskState helpers shared by the queue panel and status bar.
// TaskState serializes from Rust as either a bare lowercase string
// ("queued" | "running" | "paused" | "cancelled" | "completed") or, for the
// two data-carrying variants, a single-key object: { failed: "message" } /
// { retrying: 2 }.

export function formatSize(bytes) {
  if (bytes === null || bytes === undefined) return "";
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  if (bytes < 1024 * 1024 * 1024) return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
  return `${(bytes / (1024 * 1024 * 1024)).toFixed(2)} GB`;
}

export function formatTime(unixSecs) {
  if (unixSecs === null || unixSecs === undefined) return "";
  const d = new Date(unixSecs * 1000);
  if (Number.isNaN(d.getTime())) return "";
  const pad = (n) => String(n).padStart(2, "0");
  return `${pad(d.getDate())}.${pad(d.getMonth() + 1)}.${d.getFullYear()} ${pad(d.getHours())}:${pad(d.getMinutes())}`;
}

export function taskStateKind(state) {
  if (typeof state === "string") return state;
  if (state && typeof state === "object") return Object.keys(state)[0];
  return "queued";
}

export function taskStateDetail(state) {
  if (state && typeof state === "object") return Object.values(state)[0];
  return null;
}

export function progressPct(task) {
  if (!task.totalBytes) return 0;
  return Math.min(100, (task.transferredBytes / task.totalBytes) * 100);
}
