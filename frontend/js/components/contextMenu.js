import { iconMarkup } from "../icons.js";

let openMenuEl = null;

function closeOpenMenu() {
  if (openMenuEl) {
    openMenuEl.remove();
    openMenuEl = null;
    document.removeEventListener("mousedown", onOutsideClick, true);
    document.removeEventListener("keydown", onKeydown, true);
  }
}

function onOutsideClick(e) {
  if (openMenuEl && !openMenuEl.contains(e.target)) closeOpenMenu();
}

function onKeydown(e) {
  if (e.key === "Escape") closeOpenMenu();
}

/**
 * items: Array<{ label, icon?, danger?, onClick } | 'sep'>
 */
export function showContextMenu(x, y, items) {
  closeOpenMenu();
  const menu = document.createElement("div");
  menu.className = "context-menu";
  for (const item of items) {
    if (item === "sep") {
      const sep = document.createElement("div");
      sep.className = "context-menu-sep";
      menu.appendChild(sep);
      continue;
    }
    const btn = document.createElement("button");
    btn.className = "context-menu-item" + (item.danger ? " danger" : "");
    btn.innerHTML = `${item.icon ? iconMarkup(item.icon, 14) : ""}<span>${item.label}</span>`;
    btn.addEventListener("click", () => {
      closeOpenMenu();
      item.onClick();
    });
    menu.appendChild(btn);
  }
  document.body.appendChild(menu);

  const rect = menu.getBoundingClientRect();
  const vw = window.innerWidth;
  const vh = window.innerHeight;
  menu.style.left = `${Math.min(x, vw - rect.width - 8)}px`;
  menu.style.top = `${Math.min(y, vh - rect.height - 8)}px`;

  openMenuEl = menu;
  setTimeout(() => {
    document.addEventListener("mousedown", onOutsideClick, true);
    document.addEventListener("keydown", onKeydown, true);
  }, 0);
}
