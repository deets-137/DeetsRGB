# DeetsRGB — architecture

## Shape

A single frameless 360×560 panel that lives in the tray. Left-click on the tray
icon toggles it (positioned with its bottom-right corner at the click, clamped
to the monitor); it hides on focus loss in release builds and on `×` / Escape.
Right-click opens the tray menu: one item per Scene, then *Open* and *Quit*.
Closing the panel never quits — the tray menu owns Quit.

## Back-end (`src-tauri/src/`)

| File | Owns |
|---|---|
| `lib.rs` | Tauri setup, the tray, panel show/hide/toggle, and every `#[tauri::command]`. Holds one `HidApi` (re-enumerated on every call so re-plugged devices just work) and a `HiddenAt` timestamp so the tray click that blurred the panel doesn't immediately re-open it. |
| `sinowealth.rs` | The keyboard driver: open the `FF00`/`0x01` vendor collection, read/write the config block and colour table with marker checks, `status` / `set_solid` / `set_off`. |
| `razer.rs` | The mouse driver: the 90-byte Razer report on interface 0, transaction id `0x3F`, extended-matrix static / brightness / off on the logo and scroll-wheel LEDs. |
| `scenes.rs` | `Light`, `Scene`, and the JSON store (`%APPDATA%/com.deetsrgb.app/deetsrgb.json`) that keeps the scenes plus the last state DeetsRGB applied to each device. |

**Launch at startup** is the `DeetsRGB` value under
`HKCU\Software\Microsoft\Windows\CurrentVersion\Run`, written with `reg.exe`
(no crate). The first run of an installed build enrols once (`autostart_seeded`
in the store); the title-menu toggle reads and writes the key directly after
that, so the registry — not the store — is the source of truth. Dev builds
never touch it.

Brightness crosses the boundary as 0–100 and is mapped per device in `lib.rs`
(keyboard 0–9, mouse 0–255). The mouse cannot report its colour back, which is
why the store remembers what was last applied.

## Front-end (`src/`)

| File | Owns |
|---|---|
| `main.ts` | Boot: theme, the two device cards, scenes, the settings menu, chrome, and re-probing devices whenever the panel gains focus. |
| `device.ts` | One card = one `Light`: swatches, native colour input, brightness slider, Apply / Off. Nothing is sent until Apply. |
| `api.ts` | Typed `invoke` wrappers; the only file that names a command. |
| `theme.ts` | Copied from DeetsMusic — shared `deets.theme` key, `RETIRED` map. |
| `styles.css` | Chrome (DeetsMusic lineage) + the cards. Tokens only. |
| `styles/` | `palette.css` / `themes.css` verbatim from DeetsMusic; `skin.css` = base + Press; `fonts.css` = Anton + IBM Plex Mono. |

## Not built (see PLAN.md)

Keyboard stock effects (speed / colour per effect), per-key "Custom picture",
mouse spectrum / breathing, and a keyboard-shortcut for scenes.
