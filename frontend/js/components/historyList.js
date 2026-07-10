// Shared "recent connections" row renderer used by both the welcome screen
// and the toolbar's History popover.
import { iconMarkup } from "../icons.js";
import { t } from "../i18n.js";
import { showContextMenu } from "./contextMenu.js";
import { reconnectFromHistory, saveHistoryAsSite } from "../connectActions.js";
import { openForEdit } from "./connectDialog.js";

/** entries: HistoryEntry[]; rowClass defaults to "recent-row". */
export function renderHistoryRows(container, entries, { rowClass = "recent-row", onAfterClick } = {}) {
  container.innerHTML = "";
  for (const entry of entries) {
    const row = document.createElement("div");
    row.className = rowClass;
    row.innerHTML = `
      <span class="icon">${iconMarkup("serverSquare", 13)}</span>
      <span class="recent-label">${entry.username}@${entry.host}:${entry.port}</span>
      <span class="recent-time">${entry.connectedAt}</span>
    `;
    row.addEventListener("click", () => {
      reconnectFromHistory(entry);
      onAfterClick?.();
    });
    row.addEventListener("contextmenu", (e) => {
      e.preventDefault();
      showContextMenu(e.clientX, e.clientY, [
        {
          label: t("common.edit"),
          icon: "pen",
          onClick: () => {
            openForEdit(entry);
            onAfterClick?.();
          },
        },
        {
          label: t("common.save"),
          icon: "star",
          onClick: () => {
            saveHistoryAsSite(entry);
            onAfterClick?.();
          },
        },
      ]);
    });
    container.appendChild(row);
  }
}
