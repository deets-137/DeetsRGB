# Wire protocols

Both devices are driven with HID **feature reports** — no kernel driver, no
vendor software. Everything here was verified on the actual hardware on
2026-09-03 with the Python scratch tools noted at the bottom.

---

## 1. BY Tech / Sinowealth SH68F90A keyboard — `258A:010C`

USB product string is literally "Gaming Keyboard", manufacturer "BY Tech".
The same firmware family ships as the AULA F75, Portronics Hydra 10, and a long
tail of unbranded boards; the MCU is Sinowealth's 8051-based SH68F90A.

### Interface

Interface 1 exposes seven collections. The lighting channel is the
**usage page `0xFF00`, usage `0x0001` collection whose only report is a
519-byte FEATURE with report ID 6** (`Col06` in the Windows path). Report 5 (a
5-byte feature) and report 3 (a 3-byte input) on the same page are separate
vendor channels and are not used.

### Packet

520 bytes, always:

```
06 CMD 00 00 01 00 L0 L1 <data …padded with zeros>
```

`CMD | 0x80` is the read form. `L0 L1` is the payload length, little-endian.
Bytes 2–5 are fixed for single-page transfers (the configurator overrides 4–5
with page count / page index for multi-page macro writes only).

| Write | Read | Region | Length |
|---|---|---|---|
| `0x04` | `0x84` | Config block | `0x0080` |
| `0x0A` | `0x8A` | Colour table | `0x0200` |
| `0x03` | `0x83` | Keymap (3 layers × 4 bytes/key) | — |
| `0x05` | `0x85` | Macros (512-byte pages) | — |
| `0x06` | `0x86` | Per-key lighting (3 × 126-byte R/G/B planes) | `0x0180` |
| `0x08` | — | Live LED stream (interleaved RGB, not persisted) | `0x017A` |

**Reading** = send the read-form packet as a SET_FEATURE, wait ~50 ms, then
GET_FEATURE report 6; the answer echoes the header and carries the block. A
busy device answers zeros — retry (up to 4 ×, 40 ms apart) until the config
marker is present.

### Config block (128 bytes)

| Offset | Field |
|---|---|
| `0x01` | Poll rate: 1 = 250 Hz, 2 = 500 Hz, 3 = 1000 Hz |
| `0x03` | Low-latency mode: 0 = on, 2 = off |
| `0x09` | Light type: 0 = preset effects, 1 = per-key ("Custom picture") |
| `0x0A` | **Current effect id** |
| `0x0F` | Win-key lock |
| `0x18` | Sleep timer, minutes × 2 (0 = off) |
| `0x1B` | macOS mode |
| `0x1C`–`0x1F` | Indicator strip: mode, colour preset, brightness, ? |
| `0x3A`–`0x5B` | **Effect table** — 2 bytes per effect at fixed, gappy offsets |
| `0x7E`–`0x7F` | **Marker `5A A5`** — must be present on read; never write without it |

Effect entry = `[brightness 0–9][speed << 4 | colour]`. Speed 0–4 (the firmware
clamps higher values). Colour nibble: `0` = the effect's user-RGB slot,
`1` green, `2` blue, `3` yellow, `4` magenta, `5` cyan, `6` white, `7` multi/rainbow.

| Effect | Name | Offset | Effect | Name | Offset |
|---|---|---|---|---|---|
| 0 | Off | — | 10 | Continuous stream | `0x4C` |
| 1 | **Solid** | `0x3A` | 11 | Stream (factory default) | `0x4E` |
| 2 | Breathing | `0x3C` | 12 | Shadow | `0x50` |
| 3 | Rainbow | `0x3E` | 13 | Sine wave | `0x52` |
| 4 | Flicker | `0x40` | 15 | Pinwheel | `0x54` |
| 5 | Running rain | `0x42` | 16 | Waterfall (per-key) | `0x58` |
| 7 | Ripple | `0x46` | 17 | Bloom (per-key) | `0x5A` |
| 8 | Stars | `0x48` | 21 | Custom picture (per-key) | — |

Ids 6, 9, 14, 18–20 do not exist; **do not compute an offset for them**.

### Colour table (512 bytes)

24 blocks × 21 bytes = 7 RGB slots per block; **block index = effect id**.
Slot 0 is the user colour (factory red), slots 1–6 are the fixed presets
green, blue, yellow, magenta, cyan, white. So Solid's user colour lives at
bytes 21–23. Per-key effects (16, 17, 21) do not read this table.

### To set a solid colour (what `sinowealth.rs` does)

1. Read colour table (`0x8A`) → write slot 0 of block 1 → write back (`0x0A`).
2. Read config (`0x84`, verify marker) → `0x0A = 1`, `0x09 = 0`, `0x3A = brightness`, `0x3B = 0x00` → write back (`0x04`), wait 60 ms.
3. Re-read to confirm. The keyboard stores this itself; it survives unplug / reboot.

Off = `0x0A = 0`. The `0x08` live stream is **ignored** unless a per-key mode
is active — that was the first thing tried, and nothing happened.

### Sources

- [zerom-code/gx87-studio](https://github.com/zerom-code/gx87-studio) — `docs/PROTOCOL_RU.md`, `Vendor/SinowealthProtocol.cs`, `Localization/T.Names.cs` (the byte map, the effect names, the write procedure).
- [xevrion — Reverse engineering the AULA F75](https://xevrion.dev/blogs/aula-f75-linux-reverse-engineering) — same PID; the header format and the ripple entry at 78–79 that confirmed the map.
- [MRtojisan/portronics-hydra-10-SignalRGB-Plugin](https://github.com/MRtojisan/portronics-hydra-10-SignalRGB-Plugin) — same PID; the `0x08` stream format.
- OpenRGB's Sinowealth keyboard support is **disabled** upstream: a Redragon board sharing a PID was bricked by it. Hence the exact-match rule in `CLAUDE.md`.
- Recovery, if it ever comes to it: [carlossless/sinowisp](https://github.com/carlossless/sinowealth-kb-tool) flashes SH68F90A parts through their ISP bootloader.

---

## 2. Razer DeathAdder Elite — `1532:005C`

### Interface

Interface 0 (the mouse collection, usage page 1 / usage 2). Windows will not
give a mouse collection read/write access, but feature reports still go
through — hidapi opens it with no access flags and `HidD_SetFeature` /
`HidD_GetFeature` work. Interfaces 1 and 2 do not answer.

### Report

90 bytes, sent as feature report ID 0 (so a 91-byte buffer on Windows):

```
[0] status   [1] transaction id   [2..3] remaining packets   [4] protocol
[5] data size   [6] command class   [7] command id   [8..87] args   [88] crc   [89] 0
```

`crc` = XOR of bytes 2–87. Send, wait ~8 ms, GET_FEATURE the same size; the
reply's `status` is `0x02` on success (`0x01` busy, `0x03` failure,
`0x05` not supported). The Elite wants **transaction id `0x3F`** (firmware
answered `v1.6`).

### Commands (extended matrix, class `0x0F`)

| Class / cmd / size | Args | Meaning |
|---|---|---|
| `00 / 81 / 02` | — | Firmware version → args[0].args[1] |
| `0F / 02 / 09` | `01 LED 01 00 00 01 R G B` | **Static** colour (VARSTORE) |
| `0F / 02 / 06` | `01 LED 00 00 00 00` | Effect **none** (off) |
| `0F / 02 / 06` | `01 LED 03 00 00 00` | Spectrum cycle |
| `0F / 02 / 09` | `01 LED 02 01 00 01 R G B` | Breathing, single colour |
| `0F / 04 / 03` | `01 LED B` | **Brightness** 0–255 (VARSTORE) |
| `0F / 84 / 03` | `01 LED 00` | Read brightness → args[2] |

LED ids: scroll wheel `0x01`, logo `0x04`. `VARSTORE` (`0x01`) makes the mouse
keep the setting; `NOSTORE` (`0x00`) would be session-only.

### Sources

- [openrazer](https://github.com/openrazer/openrazer) — `driver/razermouse_driver.c` (the Elite's cases), `razerchromacommon.c` (arg layouts), `razercommon.c` (crc, control-message shape).
- [OpenRGB](https://gitlab.com/CalcProgrammer1/OpenRGB) — `RazerController.cpp` (feature-report transport on Windows), `RazerDevices.cpp` (Elite = extended matrix, txn `0x3F`, logo + scroll zones).

---

## Scratch tools

Kept outside the repo (session scratchpad) but worth recreating when probing:
`kbled.py` (`status | solid R G B [0-9] | restore cfg.bin ct.bin`) and
`razer_test.py` (`set R G B brightness`). Both are ~60 lines on `hidapi`, and
both snapshot before they write.
