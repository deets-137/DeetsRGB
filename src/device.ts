// A device card: swatches + colour input + brightness → one Light, sent on Apply.
// The card never writes to hardware as you drag; Apply (or Off) is the commit.
import { setDevice, type DeviceId, type Light, type RGB } from "./api";

// The swatch row. Cool/warm white are the desk defaults; the rest are the
// keyboard's own preset hues so the chip and the firmware agree on "red".
const SWATCHES: { name: string; rgb: RGB }[] = [
  { name: "Cool white", rgb: [220, 235, 255] },
  { name: "Warm white", rgb: [255, 214, 170] },
  { name: "Red", rgb: [255, 0, 0] },
  { name: "Green", rgb: [0, 255, 0] },
  { name: "Blue", rgb: [0, 0, 255] },
  { name: "Magenta", rgb: [255, 0, 255] },
  { name: "Cyan", rgb: [0, 255, 255] },
];

export const toHex = (rgb: RGB) =>
  "#" + rgb.map((c) => c.toString(16).padStart(2, "0")).join("").toUpperCase();

export const fromHex = (hex: string): RGB => {
  const m = /^#?([a-f\d]{2})([a-f\d]{2})([a-f\d]{2})$/i.exec(hex);
  return m ? [parseInt(m[1], 16), parseInt(m[2], 16), parseInt(m[3], 16)] : [255, 255, 255];
};

export interface DeviceCard {
  id: DeviceId;
  light(): Light;
  setLight(l: Light): void;
  setStatus(state: "ok" | "missing" | "probing", text: string): void;
}

export function makeDeviceCard(root: HTMLElement, onNotify: (msg: string, isError?: boolean) => void): DeviceCard {
  const id = root.dataset.device as DeviceId;
  const swatches = root.querySelector<HTMLElement>(".swatches")!;
  const colour = root.querySelector<HTMLInputElement>(".colour")!;
  const range = root.querySelector<HTMLInputElement>(".range")!;
  const hex = root.querySelector<HTMLElement>('[data-role="hex"]')!;
  const pct = root.querySelector<HTMLElement>('[data-role="pct"]')!;
  const status = root.querySelector<HTMLElement>(".status")!;
  const statusText = status.querySelector<HTMLElement>(".status__text")!;
  let on = true;

  const reflect = () => {
    hex.textContent = colour.value.toUpperCase();
    pct.textContent = `${range.value}%`;
    root.style.setProperty("--swatch", colour.value); // the card's live tint (see styles.css)
    swatches.querySelectorAll<HTMLElement>(".swatch").forEach((b) => {
      b.setAttribute("aria-checked", String(b.dataset.rgb === colour.value.toUpperCase()));
    });
  };

  for (const s of SWATCHES) {
    const b = document.createElement("button");
    b.type = "button";
    b.className = "swatch";
    b.title = s.name;
    b.setAttribute("role", "radio");
    b.dataset.rgb = toHex(s.rgb);
    b.style.setProperty("--swatch", toHex(s.rgb));
    b.addEventListener("click", () => {
      colour.value = toHex(s.rgb).toLowerCase();
      reflect();
    });
    swatches.appendChild(b);
  }
  colour.addEventListener("input", reflect);
  range.addEventListener("input", reflect);

  const light = (): Light => ({ on, rgb: fromHex(colour.value), brightness: Number(range.value) });

  const send = async (l: Light) => {
    root.classList.add("is-busy");
    try {
      await setDevice(id, l);
      const who = id === "keyboard" ? "Keyboard" : "Mouse";
      onNotify(l.on ? `${who} → ${toHex(l.rgb)} at ${l.brightness}%` : `${who} off`);
    } catch (e) {
      onNotify(String(e), true);
    } finally {
      root.classList.remove("is-busy");
    }
  };

  root.querySelector('[data-action="apply"]')!.addEventListener("click", () => {
    on = true;
    void send(light());
  });
  root.querySelector('[data-action="off"]')!.addEventListener("click", () => {
    void send({ ...light(), on: false });
  });

  reflect();
  return {
    id,
    light,
    setLight(l) {
      on = l.on;
      if (l.on) {
        colour.value = toHex(l.rgb).toLowerCase();
        range.value = String(l.brightness);
      }
      reflect();
    },
    setStatus(state, text) {
      status.dataset.status = state;
      statusText.textContent = text;
    },
  };
}
