// Thin rendering helper around the Solar BoldDuotone icon paths from
// icons-data.js. Every icon has two path layers: a full-opacity "detail"
// layer and a 50%-opacity "soft" background layer.
import { ICONS } from "./icons-data.js";

const softLayerCache = new Map();

function withSolidSecondaryLayer(body) {
  const cached = softLayerCache.get(body);
  if (cached) return cached;
  const processed = body.replace(/<path([^>]*)\/>/g, (match, attrs) => {
    if (!/\sopacity="/.test(attrs)) return match;
    const withoutOpacity = attrs.replace(/\s*opacity="[^"]*"/, "");
    return `<path${withoutOpacity} class="icon-soft"/>`;
  });
  softLayerCache.set(body, processed);
  return processed;
}

export function iconMarkup(name, size = 16) {
  const body = ICONS[name];
  if (!body) return "";
  return `<svg class="icon" viewBox="0 0 24 24" width="${size}" height="${size}" aria-hidden="true">${withSolidSecondaryLayer(body)}</svg>`;
}

export function iconEl(name, size = 16) {
  const wrap = document.createElement("span");
  wrap.style.display = "inline-flex";
  wrap.innerHTML = iconMarkup(name, size);
  return wrap.firstElementChild;
}

// ── File-type icon lookup ──────────────────────────────────────────────

const FILE_ICONS = {
  pdf: ["fileText", "icon--important"],
  xlsx: ["document", "icon--important"],
  xls: ["document", "icon--important"],
  doc: ["document", "icon--important"],
  docx: ["document", "icon--important"],
  png: ["gallery", "icon--important"],
  jpg: ["gallery", "icon--important"],
  jpeg: ["gallery", "icon--important"],
  gif: ["gallery", "icon--important"],
  webp: ["gallery", "icon--important"],
  svg: ["gallery", "icon--important"],
  mp4: ["videocamera", "icon--important"],
  mkv: ["videocamera", "icon--important"],
  avi: ["videocamera", "icon--important"],
  mov: ["videocamera", "icon--important"],
  webm: ["videocamera", "icon--important"],
  mp3: ["musicNote", "icon--important"],
  ogg: ["musicNote", "icon--important"],
  flac: ["musicNote", "icon--important"],
  wav: ["musicNote", "icon--important"],
  m4a: ["musicNote", "icon--important"],
  zip: ["archive", "icon--important"],
  tar: ["archive", "icon--important"],
  gz: ["archive", "icon--important"],
  bz2: ["archive", "icon--important"],
  xz: ["archive", "icon--important"],
  "7z": ["archive", "icon--important"],
  yaml: ["code", "icon--important"],
  yml: ["code", "icon--important"],
  toml: ["code", "icon--important"],
  json: ["code", "icon--important"],
  php: ["code", "icon--important"],
  java: ["code", "icon--important"],
  class: ["code", "icon--important"],
  jar: ["code", "icon--important"],
  rs: ["code", "icon--important"],
  py: ["code", "icon--important"],
  sh: ["code", "icon--important"],
  bash: ["code", "icon--important"],
  fish: ["code", "icon--important"],
  zsh: ["code", "icon--important"],
  html: ["code", "icon--important"],
  css: ["code", "icon--important"],
  js: ["code", "icon--important"],
  ts: ["code", "icon--important"],
  sql: ["database", "icon--important"],
  db: ["database", "icon--important"],
  sqlite: ["database", "icon--important"],
  env: ["shieldKeyhole", "icon--important"],
  pem: ["key", "icon--important"],
  pub: ["key", "icon--important"],
  log: ["documentText", "icon--inactive"],
  htaccess: ["shieldKeyhole", "icon--inactive"],
  conf: ["settings", "icon--inactive"],
  ini: ["settings", "icon--inactive"],
  txt: ["documentText", "icon--inactive"],
  md: ["documentText", "icon--inactive"],
};

export function fileIconFor(name) {
  const ext = name.includes(".") ? name.split(".").pop().toLowerCase() : "";
  return FILE_ICONS[ext] ?? ["file", "text-dim"];
}

// ── Quick-access sidebar icon map ──────────────────────────────────────

export const QUICK_ACCESS_ICONS = {
  Home: "home",
  Desktop: "monitor",
  Documents: "document",
  Downloads: "downloadSquare",
  Pictures: "gallery",
  Music: "musicNote",
  Videos: "videocamera",
};
