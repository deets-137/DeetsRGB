//! BY Tech / Sinowealth SH68F90A keyboard (258A:010C) — lighting over HID feature reports.
//!
//! Transport: report ID 6, 520 bytes, on the vendor collection (usage page FF00,
//! usage 1, the 519-byte FEATURE collection — `Col06` on Windows):
//!
//!     06 CMD 00 00 01 00 L0 L1 <data…>            CMD | 0x80 = read
//!     0x84 / 0x04  read / write the 128-byte config block   (5A A5 trailer at 0x7E)
//!     0x8A / 0x0A  read / write the 512-byte colour table   (24 blocks × 7 RGB slots)
//!
//! Config block: 0x09 light-type (0 preset / 1 per-key), 0x0A current effect, and a
//! per-effect entry `[brightness 0-9][speed<<4 | colour]` at a fixed (gappy) offset.
//! Colour nibble 0 = the effect's user-RGB slot (colour table block = effect id, slot 0).
//! Effect 1 is Solid, effect 0 is Off. See docs/protocol.md for the whole map.
//!
//! Reads are "set-feature the request, then get-feature the answer". Every write is a
//! read-modify-write of a marker-verified block, exactly as the working configurators do.

use hidapi::{HidApi, HidDevice};
use serde::Serialize;
use std::{thread, time::Duration};

pub const VID: u16 = 0x258A;
pub const PID: u16 = 0x010C;

const REPORT_LEN: usize = 520;
const CFG_LEN: usize = 0x80;
const CT_LEN: usize = 0x200;
const PALETTE_STRIDE: usize = 21;

const EFFECT_OFF: u8 = 0;
const EFFECT_SOLID: u8 = 1;
const SOLID_ENTRY: usize = 0x3A;
const OFF_LIGHT_TYPE: usize = 0x09;
const OFF_EFFECT: usize = 0x0A;

pub const EFFECT_NAMES: &[(u8, &str)] = &[
    (0, "Off"),
    (1, "Solid"),
    (2, "Breathing"),
    (3, "Rainbow"),
    (4, "Flicker"),
    (5, "Running rain"),
    (7, "Ripple"),
    (8, "Stars"),
    (10, "Continuous stream"),
    (11, "Stream"),
    (12, "Shadow"),
    (13, "Sine wave"),
    (15, "Pinwheel"),
    (16, "Waterfall"),
    (17, "Bloom"),
    (21, "Custom picture"),
];

#[derive(Serialize, Clone, Debug)]
pub struct KeyboardStatus {
    pub effect: u8,
    pub effect_name: String,
    /// 0–9, the firmware's own scale (the Solid effect's entry).
    pub brightness: u8,
    /// The Solid effect's user colour (colour-table block 1, slot 0).
    pub rgb: [u8; 3],
}

pub struct Keyboard {
    dev: HidDevice,
}

impl Keyboard {
    pub fn open(api: &HidApi) -> Option<Keyboard> {
        let info = api.device_list().find(|d| {
            d.vendor_id() == VID
                && d.product_id() == PID
                && d.usage_page() == 0xFF00
                && d.usage() == 0x0001
                && d.path().to_string_lossy().contains("Col06")
        })?;
        let dev = info.open_device(api).ok()?;
        Some(Keyboard { dev })
    }

    fn packet(cmd: u8, len: usize, data: &[u8]) -> Vec<u8> {
        let mut p = vec![0u8; REPORT_LEN];
        p[0] = 0x06;
        p[1] = cmd;
        p[4] = 0x01;
        p[6] = (len & 0xFF) as u8;
        p[7] = (len >> 8) as u8;
        p[8..8 + data.len()].copy_from_slice(data);
        p
    }

    fn read_block(&self, cmd: u8, len: usize, check_marker: bool) -> Result<Vec<u8>, String> {
        // A busy device answers with zeros; the GX87 configurator retries the same way.
        for _ in 0..4 {
            self.dev
                .send_feature_report(&Self::packet(cmd, len, &[]))
                .map_err(|e| format!("keyboard read request: {e}"))?;
            thread::sleep(Duration::from_millis(50));
            let mut buf = vec![0u8; REPORT_LEN + 1];
            buf[0] = 0x06;
            self.dev
                .get_feature_report(&mut buf)
                .map_err(|e| format!("keyboard read: {e}"))?;
            let data = buf[8..8 + len].to_vec();
            if !check_marker || data[0x7E..0x80] == [0x5A, 0xA5] {
                return Ok(data);
            }
            thread::sleep(Duration::from_millis(40));
        }
        Err("keyboard config read failed the 5A A5 marker check; refusing to write".into())
    }

    fn write_block(&self, cmd: u8, data: &[u8]) -> Result<(), String> {
        self.dev
            .send_feature_report(&Self::packet(cmd, data.len(), data))
            .map_err(|e| format!("keyboard write: {e}"))?;
        thread::sleep(Duration::from_millis(60)); // firmware settle, per GX87
        Ok(())
    }

    pub fn status(&self) -> Result<KeyboardStatus, String> {
        let cfg = self.read_block(0x84, CFG_LEN, true)?;
        let ct = self.read_block(0x8A, CT_LEN, false)?;
        let effect = cfg[OFF_EFFECT];
        let name = EFFECT_NAMES
            .iter()
            .find(|(id, _)| *id == effect)
            .map(|(_, n)| n.to_string())
            .unwrap_or_else(|| format!("Effect {effect}"));
        let o = (EFFECT_SOLID as usize) * PALETTE_STRIDE;
        Ok(KeyboardStatus {
            effect,
            effect_name: name,
            brightness: cfg[SOLID_ENTRY],
            rgb: [ct[o], ct[o + 1], ct[o + 2]],
        })
    }

    /// Solid colour, persisted in the keyboard's own flash (survives unplug/reboot).
    pub fn set_solid(&self, rgb: [u8; 3], brightness: u8) -> Result<(), String> {
        let mut ct = self.read_block(0x8A, CT_LEN, false)?;
        let o = (EFFECT_SOLID as usize) * PALETTE_STRIDE;
        ct[o..o + 3].copy_from_slice(&rgb);
        self.write_block(0x0A, &ct)?;

        let mut cfg = self.read_block(0x84, CFG_LEN, true)?;
        cfg[OFF_EFFECT] = EFFECT_SOLID;
        cfg[OFF_LIGHT_TYPE] = 0;
        cfg[SOLID_ENTRY] = brightness.min(9);
        cfg[SOLID_ENTRY + 1] = 0x00; // speed 0, colour nibble 0 = user RGB
        self.write_block(0x04, &cfg)
    }

    pub fn set_off(&self) -> Result<(), String> {
        let mut cfg = self.read_block(0x84, CFG_LEN, true)?;
        cfg[OFF_EFFECT] = EFFECT_OFF;
        cfg[OFF_LIGHT_TYPE] = 0;
        self.write_block(0x04, &cfg)
    }
}
