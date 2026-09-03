# DeetsRGB — roadmap (only what is *not* built)

- **Effects** — the keyboard's 15 stock effects (`docs/protocol.md`, effect
  table) with brightness / speed / colour per effect; mouse spectrum + breathing
  (`0x0F/0x02` effect ids `0x03` / `0x02`).
- **Per-key** — effect 21 "Custom picture" via opcode `0x06` (three 126-byte
  R/G/B planes) once the LED index map for this board is known; the `0x08` live
  stream only shows in a per-key mode.
- **Scene hotkeys** — a global shortcut per scene.
- **Extra keys** — the keyboard's ten extra buttons: identify what they emit
  (consumer-control report 2 / vendor report 3) and decide whether DeetsRGB
  should bind them to scenes.
