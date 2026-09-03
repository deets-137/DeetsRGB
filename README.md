# DeetsRGB

A tray lighting panel for Windows 11. Left-click the tray icon for a small
frameless panel — keyboard card, mouse card, Scenes — right-click for the
Scenes menu. No vendor software: both devices are driven over raw HID feature
reports from a ~200-line Rust back-end, and the keyboard's protocol was
reverse-engineered on the desk it sits on.

| Device | IDs | Control |
|---|---|---|
| BY Tech / Sinowealth SH68F90A keyboard — the unbranded "Gaming Keyboard" sold under AULA, Portronics, and a long tail of other names | `258A:010C` | Solid colour + brightness, stored in the keyboard's own flash |
| Razer DeathAdder Elite | `1532:005C` | Logo + scroll-wheel colour and brightness, stored on the mouse |

## Why it exists

The keyboard shipped with no software, no brand, and Fn combos that did
nothing; OpenRGB doesn't know its PID (and has its Sinowealth driver disabled
after a bricking incident on a look-alike board). The only way to get a
40 %-brightness cool white was to find the wire protocol. Once it was found,
a tray app was the obvious place to keep it.

## What it does

- **Two device cards.** Preset swatches, a native colour picker, a brightness
  slider, and explicit Apply / Off. Nothing is written to hardware while a
  slider drags — every keyboard write is a flash write, so the commit is
  deliberate.
- **Scenes.** Named pairs of keyboard + mouse state, applied from the panel or
  straight from the tray menu. Ships with *Cool White 40 %*, *Warm White 40 %*,
  and *Lights Off*; "Save current…" captures whatever the cards show.
- **Launch at startup**, via the per-user `Run` registry key — enrolled once on
  the first installed launch, and a toggle in the title menu after that.
- **Safe writes.** The keyboard's config block carries a `5A A5` marker; the
  driver reads it before every write and refuses to write a block that lacks
  it (a busy device answers with zeros, and writing those back would erase
  every setting). Every write is read-modify-write.

## Protocols

[`docs/protocol.md`](docs/protocol.md) is the byte-level reference for both
devices — packet shapes, the keyboard's config map and colour table, the
Razer report and CRC — with the sources each fact was pieced together from.
If you have a `258A:010C` board of your own, that document is the useful part
of this repo.

## Run

```bash
npm install
npm run tauri dev
```

First run compiles Rust and is slow. The app starts hidden in the tray.
`npm run release` produces an NSIS installer under
`src-tauri/target/release/bundle/nsis/`.

## Family

Part of the Deets family and sharing its design language:
[DeetsMusic](https://github.com/deets-137/DeetsMusic) (the token system
originates there), [DeetsSolutions](https://github.com/deets-137/DeetsSolutions),
[DeetsFilm](https://github.com/deets-137/DeetsFilm),
[DeetsSQL](https://github.com/deets-137/DeetsSQL). DeetsRGB ships the Press
skin only, like DeetsFilm, and stores its theme under the family's
`deets.theme` key.
