/** Renders a `/`-separated path as clickable breadcrumb segments into `container`. */
export function renderBreadcrumbs(container, path, onNavigate) {
  container.innerHTML = "";
  const parts = path.split("/").filter(Boolean);

  const root = document.createElement("button");
  root.type = "button";
  root.className = "breadcrumb-seg";
  root.textContent = "/";
  root.addEventListener("click", () => onNavigate("/"));
  container.appendChild(root);

  let acc = "";
  parts.forEach((part, i) => {
    acc += `/${part}`;
    const isLast = i === parts.length - 1;
    const seg = document.createElement(isLast ? "span" : "button");
    seg.className = `breadcrumb-seg ${isLast ? "current" : ""}`;
    seg.textContent = part;
    if (!isLast) {
      const target = acc;
      seg.type = "button";
      seg.addEventListener("click", () => onNavigate(target));
    }
    container.appendChild(seg);
    if (!isLast) {
      const sep = document.createElement("span");
      sep.className = "breadcrumb-sep";
      sep.textContent = "/";
      container.appendChild(sep);
    }
  });
}
