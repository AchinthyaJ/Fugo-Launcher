/** Launcher appearance: background themes and cursor packs, saved per viewer on this machine. */

export interface BackgroundTheme {
  id: string;
  label: string;
  src: string;
}

/** Built-in backgrounds. More land here as they are added. */
export const BACKGROUNDS: BackgroundTheme[] = [{ id: "default", label: "Default", src: "/assets/backgrounds/1.png" }];

export interface CursorPack {
  id: string;
  label: string;
  /** Folder under public/ with the PNGs, or null for the system cursor. */
  dir: string | null;
  /** Hotspots in pixels for each PNG (the tip of the cursor). */
  hotspots: Record<string, { x: number; y: number }>;
}

export const CURSOR_PACKS: CursorPack[] = [
  { id: "system", label: "System default", dir: null, hotspots: {} },
  {
    id: "skyrim",
    label: "Skyrim",
    dir: "/cursors/skyrim",
    hotspots: {
      normal: { x: 1, y: 1 },
      pointer: { x: 16, y: 1 },
      text: { x: 15, y: 13 },
      move: { x: 15, y: 13 },
      help: { x: 1, y: 1 },
      precision: { x: 15, y: 15 },
      "not-allowed": { x: 1, y: 1 },
      "resize-ns": { x: 16, y: 16 },
      "resize-ew": { x: 16, y: 16 },
    },
  },
];

export interface Prefs {
  background: string;
  customBackground: string | null;
  cursorPack: string;
}

const KEY = "fugo.customize";
/** Largest custom background kept in local storage (the data URL is stored as text). */
export const MAX_CUSTOM_BYTES = 2 * 1024 * 1024;

export function loadPrefs(): Prefs {
  const fallback: Prefs = { background: "default", customBackground: null, cursorPack: "system" };
  try {
    const saved = JSON.parse(localStorage.getItem(KEY) || "null");
    return { ...fallback, ...saved };
  } catch {
    return fallback;
  }
}

export function savePrefs(prefs: Prefs): void {
  try {
    localStorage.setItem(KEY, JSON.stringify(prefs));
  } catch {
    // Storage is full or blocked: the choice still applies for this session.
  }
}

export function backgroundSrc(prefs: Prefs): string {
  if (prefs.background === "custom" && prefs.customBackground) return prefs.customBackground;
  return (BACKGROUNDS.find((b) => b.id === prefs.background) ?? BACKGROUNDS[0]).src;
}

const STYLE_ID = "fugo-cursor-pack";

/** Puts the chosen cursor pack on every element. The system cursor removes the override. */
export function applyCursorPack(id: string): void {
  let style = document.getElementById(STYLE_ID) as HTMLStyleElement | null;
  const pack = CURSOR_PACKS.find((p) => p.id === id);
  if (!pack || !pack.dir) {
    style?.remove();
    return;
  }
  if (!style) {
    style = document.createElement("style");
    style.id = STYLE_ID;
    document.head.appendChild(style);
  }
  const url = (name: string) => `url("${pack.dir}/${name}.png")`;
  const hot = (name: string) => {
    const h = pack.hotspots[name] ?? { x: 0, y: 0 };
    return `${url(name)} ${h.x} ${h.y}`;
  };
  style.textContent = `
    html, body, * { cursor: ${hot("normal")}, auto !important; }
    a, button, [role="button"], [role="option"], [role="tab"], label, .cursor-pointer {
      cursor: ${hot("pointer")}, pointer !important;
    }
    input[type="text"], input[type="search"], input[type="password"], textarea, [contenteditable="true"] {
      cursor: ${hot("text")}, text !important;
    }
    [aria-disabled="true"], :disabled { cursor: ${hot("not-allowed")}, not-allowed !important; }
  `;
}

/** An image at a URL on this site, re-encoded as a PNG data URL (the skin service only takes PNG). */
export async function urlToDataUrl(url: string): Promise<string> {
  const blob = await fetch(url).then((r) => r.blob());
  const bitmap = await createImageBitmap(blob);
  const canvas = document.createElement("canvas");
  canvas.width = bitmap.width;
  canvas.height = bitmap.height;
  const ctx = canvas.getContext("2d");
  if (!ctx) throw new Error("Could not read the cape.");
  ctx.imageSmoothingEnabled = false;
  ctx.drawImage(bitmap, 0, 0);
  bitmap.close();
  return canvas.toDataURL("image/png");
}
