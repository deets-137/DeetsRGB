// Theme switching. The whole mechanism is one attribute on <html>;
// the token sheets (themes.css) do the rest. Choice persists in localStorage.

export type ThemeName = "lilac" | "green" | "sepia" | "moonlight" | "black-yellow" | "black-red";

const STORAGE_KEY = "deets.theme";

// Retired ids still sitting in a saved localStorage value, mapped to their
// successor. The 2026-08-10 rename traded four vibe names for what the
// themes actually are; without this map anyone who had picked one silently
// falls back to the default instead of keeping their choice.
// Kept in sync with the same map in skin.ts and with DeetsSolutions'
// `RETIRED` in js/controls.js — the id is a contract with stored state.
const RETIRED: Record<string, ThemeName> = {
  fairy: "lilac",
  glade: "green",
  hornet: "black-yellow",
  viper: "black-red",
};

// No saved choice: Sepia. DeetsRGB deviates from the family's OS light/dark
// default (DeetsMusic lands on Lilac or Black & Red) — a tray panel that pops
// over whatever is on screen reads best as one fixed, warm stock. Kept in
// sync with the pre-paint script in index.html.
function defaultTheme(): ThemeName {
  return "sepia";
}

export function applyTheme(name: ThemeName): void {
  document.documentElement.dataset.theme = name;
  try {
    localStorage.setItem(STORAGE_KEY, name);
  } catch {
    /* private mode / storage disabled — theme still applies for the session */
  }
  markActive(name);
}

export function initTheme(): void {
  const saved = localStorage.getItem(STORAGE_KEY);
  const name = saved ? RETIRED[saved] ?? (saved as ThemeName) : defaultTheme();
  // Write back through applyTheme so a migrated id is PERSISTED, not
  // re-resolved on every launch.
  applyTheme(name);
}

// Reflect the active theme onto the flyout's radio items.
function markActive(name: string): void {
  document.querySelectorAll<HTMLElement>("[data-theme-choice]").forEach((el) => {
    el.setAttribute("aria-checked", String(el.dataset.themeChoice === name));
  });
}
