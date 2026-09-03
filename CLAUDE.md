# DeetsRGB — project guide for Claude

A tray-first lighting panel for Windows 11 (Tauri v2 + WebView2, vanilla TS
front-end, Rust back-end). It drives two devices over raw HID feature reports —
a BY Tech / Sinowealth keyboard (`258A:010C`) and a Razer DeathAdder Elite
(`1532:005C`) — and saves named Scenes that set both at once.

`README.md` is the cold start. `docs/protocol.md` is the reverse-engineered
wire format for both devices — read it before touching `sinowealth.rs` or
`razer.rs`. `docs/architecture.md` describes what exists. `PLAN.md` holds only
what is *not* built.

## Never

- **Never write a keyboard block you did not just read.** `sinowealth.rs` does
  read-modify-write on marker-verified blocks (`5A A5` at `0x7E`) and refuses to
  write when the marker is missing — a busy device answers with zeros, and
  writing those back erases every setting. This is how the working
  configurators behave, and it is the whole reason the keyboard has not bricked.
- **Never walk the keyboard's effect table linearly.** Entries exist only for
  effects 1–5, 7, 8, 10–13, 15, 16, 17 at fixed offsets with holes between them;
  a computed offset for a missing id lands on unrelated bytes.
- **Never send the Sinowealth protocol to any other PID.** OpenRGB disabled its
  Sinowealth keyboard support because a Redragon board sharing a VID/PID pair
  got bricked by it. `sinowealth.rs` matches `258A:010C` and the `FF00`/`0x01`
  `Col06` collection exactly; widen that only with a device in hand.
- **Never write to hardware while a slider drags.** Apply / Off / a Scene is
  the commit. The keyboard's config lives in its own flash; each write is a
  flash write.
- **Never write a hex code, radius, font, or duration into a component rule.**
  Colors route through the theme tier, geometry/type/motion through the skin
  tier — same discipline as DeetsMusic, DeetsSolutions, DeetsFilm, DeetsSQL.
  Every component must survive all 6 themes. (`--swatch` is the one runtime
  custom property: it carries the *device's* colour, which is content, not UI.)
- **Never add a dependency without asking.** Rust has `tauri`, `serde`,
  `serde_json`, `hidapi` and nothing else; the front-end has `@tauri-apps/api`.
  No vendor SDKs, no Synapse, no OpenRGB — the point is that this works with
  none of them installed.
- **Never rename a theme id without a `RETIRED` entry.** `deets.theme` is
  shared with DeetsMusic, DeetsSolutions, DeetsSQL and DeetsFilm — one
  appearance choice across the family. A rename lands in `src/theme.ts`, the
  pre-paint script in `index.html`, *and* the sibling repos.

## Ported code

The token CSS (`palette.css`, `themes.css`, `skin.css`, `fonts.css`), the fonts,
and `theme.ts` are **copied** from `../DeetsMusic/src/`. DeetsRGB ships the
**Press skin only** (DeetsFilm doctrine): `skin.css` keeps the base block + the
Press block, and the music-only tokens (`--np-*`, `--vol-*`, `--lib-*`,
`--album-*`, `--storm-*`, `--ocean-*`, `--scrubber-*`, `--transport-*`,
`--midi-*`, `--nav-*`) were pruned on the way over. DeetsRGB's own additions
(`--swatch-size`, `--status-dot`, `--row-label-w`, `--range-*`) sit at the end
of the base block. Values are otherwise byte-identical so a diff against the
source stays readable. When re-porting: bring the whole file, then delete.

## How to verify your work

- **The user runs the app and tests your changes** (`npm run tauri dev`) and
  gives feedback — with the actual keyboard and mouse on the desk. Do NOT build
  harnesses or mock devices.
- Cheap checks that ARE worth running: `npx tsc --noEmit`, `npx vite build`,
  and `cargo check` in `src-tauri/`.
- When probing a *new* command, reach for a ~60-line Python `hidapi` script
  first (see the bottom of `docs/protocol.md`) — a faster loop than the Tauri
  build, and it can snapshot before it writes.

## Working style

- **The user directs the architecture.** For anything non-trivial, talk it
  through first, surface the real forks (he responds well to multiple choice),
  confirm, then build.
- **Do not delegate to subagents.** The codebase is small enough to hold in
  context; read and edit it directly.
- Commit only when asked. Trailer:
  `Co-Authored-By: Claude Fable 5.1 <noreply@anthropic.com>`

## Run

```
npm install
npm run tauri dev     # compiles Rust (first run slow); the app starts in the TRAY
npx tsc --noEmit      # front-end typecheck
cd src-tauri && cargo check
```

Left-click the tray icon to open the panel (it hides on focus loss in release
builds; in dev the devtools would steal focus, so it stays). Right-click for
the Scenes menu and Quit. Devtools auto-open in dev (`src-tauri/src/lib.rs`).
