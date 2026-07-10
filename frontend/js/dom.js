/** Escapes text for safe interpolation into innerHTML template strings. */
export function escapeHtml(str) {
  const div = document.createElement("div");
  div.textContent = str ?? "";
  return div.innerHTML;
}
