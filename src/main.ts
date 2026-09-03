import { getCurrentWindow } from "@tauri-apps/api/window";
import { applyTheme, initTheme, type ThemeName } from "./theme";
import {
  appQuit,
  autostartGet,
  autostartSet,
  devicesStatus,
  panelHide,
  sceneApply,
  sceneDelete,
  sceneSave,
  scenesList,
  type Scene,
} from "./api";
import { makeDeviceCard, type DeviceCard } from "./device";

const appWindow = getCurrentWindow();

window.addEventListener("DOMContentLoaded", () => {
  initTheme();

  // ── toast (one line of feedback over the cards) ──
  const toast = document.getElementById("toast") as HTMLParagraphElement;
  let toastTimer = 0;
  const notify = (msg: string, isError = false) => {
    toast.textContent = msg;
    toast.classList.toggle("toast--error", isError);
    toast.hidden = false;
    window.clearTimeout(toastTimer);
    toastTimer = window.setTimeout(() => (toast.hidden = true), isError ? 6000 : 2500);
  };

  // ── device cards ──
  const cards: Record<string, DeviceCard> = {};
  document.querySelectorAll<HTMLElement>(".device").forEach((el) => {
    const card = makeDeviceCard(el, notify);
    cards[card.id] = card;
  });

  const refresh = async () => {
    Object.values(cards).forEach((c) => c.setStatus("probing", "Looking…"));
    try {
      const s = await devicesStatus();
      if (s.keyboard) {
        cards.keyboard.setStatus("ok", `${s.keyboard.effect_name} · ${s.keyboard.brightness}/9`);
        // Prefer what we last applied (keeps the % the user chose); fall back to the firmware's truth.
        cards.keyboard.setLight(
          s.last_keyboard ?? {
            on: s.keyboard.effect !== 0,
            rgb: s.keyboard.rgb,
            brightness: Math.round((s.keyboard.brightness / 9) * 100),
          },
        );
      } else {
        cards.keyboard.setStatus("missing", "Not found");
        if (s.keyboard_error) console.warn("[keyboard]", s.keyboard_error);
      }
      if (s.mouse) {
        cards.mouse.setStatus("ok", `fw ${s.mouse.firmware} · ${Math.round((s.mouse.logo_brightness / 255) * 100)}%`);
        if (s.last_mouse) cards.mouse.setLight(s.last_mouse);
      } else {
        cards.mouse.setStatus("missing", "Not found");
        if (s.mouse_error) console.warn("[mouse]", s.mouse_error);
      }
    } catch (e) {
      notify(String(e), true);
    }
  };

  // ── scenes ──
  const chips = document.getElementById("scene-chips")!;
  const form = document.getElementById("scene-form") as HTMLFormElement;
  const nameInput = document.getElementById("scene-name") as HTMLInputElement;

  const renderScenes = (scenes: Scene[]) => {
    chips.replaceChildren(
      ...scenes.map((s) => {
        const b = document.createElement("button");
        b.type = "button";
        b.className = "chip";
        b.textContent = s.name;
        b.setAttribute("role", "listitem");
        b.title = "Click to apply · right-click to delete";
        b.addEventListener("click", async () => {
          b.classList.add("is-busy");
          try {
            await sceneApply(s.id);
            notify(`Scene: ${s.name}`);
            void refresh();
          } catch (e) {
            notify(String(e), true);
          } finally {
            b.classList.remove("is-busy");
          }
        });
        b.addEventListener("contextmenu", async (e) => {
          e.preventDefault();
          if (!confirm(`Delete scene "${s.name}"?`)) return;
          renderScenes(await sceneDelete(s.id));
        });
        return b;
      }),
    );
  };

  document.getElementById("scene-save")!.addEventListener("click", () => {
    form.hidden = false;
    nameInput.focus();
  });
  document.getElementById("scene-cancel")!.addEventListener("click", () => {
    form.hidden = true;
  });
  form.addEventListener("submit", async (e) => {
    e.preventDefault();
    const name = nameInput.value.trim();
    if (!name) return;
    const id =
      name
        .toLowerCase()
        .replace(/[^a-z0-9]+/g, "-")
        .replace(/(^-|-$)/g, "") || `scene-${Date.now()}`;
    try {
      renderScenes(await sceneSave({ id, name, keyboard: cards.keyboard.light(), mouse: cards.mouse.light() }));
      notify(`Saved "${name}"`);
      form.hidden = true;
      nameInput.value = "";
    } catch (err) {
      notify(String(err), true);
    }
  });

  // ── chrome ──
  document.getElementById("tl-close")?.addEventListener("click", () => void panelHide());
  document.getElementById("quit")?.addEventListener("click", () => void appQuit());
  document.getElementById("refresh")?.addEventListener("click", () => {
    closeMenu();
    void refresh();
  });

  // ── Launch at startup (toggle row; the HKCU Run key is the source of truth) ──
  const autostartToggle = document.getElementById("autostart-toggle")!;
  void autostartGet().then((on) => autostartToggle.setAttribute("aria-checked", String(on)));
  autostartToggle.addEventListener("click", async (e) => {
    e.stopPropagation(); // keep the menu open so the dot feedback is visible
    const next = autostartToggle.getAttribute("aria-checked") !== "true";
    try {
      autostartToggle.setAttribute("aria-checked", String(await autostartSet(next)));
    } catch (err) {
      notify(String(err), true);
    }
  });

  // Settings menu: click to toggle, click-away / Escape to close.
  const trigger = document.getElementById("settings-trigger")!;
  const menu = document.getElementById("settings-menu")!;
  const closeMenu = () => {
    menu.hidden = true;
    trigger.setAttribute("aria-expanded", "false");
  };
  trigger.addEventListener("click", (e) => {
    e.stopPropagation();
    menu.hidden = !menu.hidden;
    trigger.setAttribute("aria-expanded", String(!menu.hidden));
  });
  document.addEventListener("click", (e) => {
    if (!menu.hidden && !menu.contains(e.target as Node)) closeMenu();
  });
  document.addEventListener("keydown", (e) => {
    if (e.key === "Escape") {
      if (!menu.hidden) closeMenu();
      else void panelHide();
    }
  });
  document.querySelectorAll<HTMLElement>("[data-theme-choice]").forEach((el) => {
    el.addEventListener("click", () => {
      applyTheme(el.dataset.themeChoice as ThemeName);
      closeMenu();
    });
  });

  // Re-probe every time the panel is shown, so a re-plugged device is picked up.
  void appWindow.onFocusChanged(({ payload: focused }) => {
    if (focused) void refresh();
  });

  void refresh();
  void scenesList().then(renderScenes);
});
