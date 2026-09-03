//! Razer DeathAdder Elite (1532:005C) — the classic 90-byte Razer report over HID.
//!
//! Sent as a feature report (report ID 0) on interface 0 — the mouse collection —
//! which is what OpenRGB does on Windows and what worked from Python here. Layout:
//!
//!     [status][txn id][remaining ×2][proto][data size][class][cmd][args ×80][crc][0]
//!
//! CRC is the XOR of bytes 2..88. The Elite wants transaction id 0x3F and the
//! "extended matrix" command set: class 0x0F, cmd 0x02 = effect, 0x04 = brightness
//! (0x84 reads it back). LED ids: scroll wheel 0x01, logo 0x04. VARSTORE (0x01) makes
//! the mouse keep the setting itself. Status 0x02 in the reply means accepted.

use hidapi::{HidApi, HidDevice};
use serde::Serialize;
use std::{thread, time::Duration};

pub const VID: u16 = 0x1532;
pub const PID: u16 = 0x005C;

const TXN: u8 = 0x3F;
const VARSTORE: u8 = 0x01;
pub const LED_SCROLL: u8 = 0x01;
pub const LED_LOGO: u8 = 0x04;
const STATUS_OK: u8 = 0x02;

#[derive(Serialize, Clone, Debug)]
pub struct MouseStatus {
    pub firmware: String,
    /// 0–255 per LED, read back from the mouse.
    pub logo_brightness: u8,
    pub scroll_brightness: u8,
}

pub struct Mouse {
    dev: HidDevice,
}

impl Mouse {
    pub fn open(api: &HidApi) -> Option<Mouse> {
        let info = api
            .device_list()
            .find(|d| d.vendor_id() == VID && d.product_id() == PID && d.interface_number() == 0)?;
        let dev = info.open_device(api).ok()?;
        Some(Mouse { dev })
    }

    fn report(class: u8, cmd: u8, size: u8, args: &[u8]) -> [u8; 91] {
        let mut r = [0u8; 91]; // r[0] = report ID 0; the 90-byte report follows
        r[2] = TXN;
        r[6] = size;
        r[7] = class;
        r[8] = cmd;
        r[9..9 + args.len()].copy_from_slice(args);
        let crc = r[3..89].iter().fold(0u8, |a, b| a ^ b);
        r[89] = crc;
        r
    }

    fn transact(&self, rep: &[u8; 91]) -> Result<[u8; 91], String> {
        self.dev
            .send_feature_report(rep)
            .map_err(|e| format!("mouse send: {e}"))?;
        thread::sleep(Duration::from_millis(8));
        let mut resp = [0u8; 91];
        self.dev
            .get_feature_report(&mut resp)
            .map_err(|e| format!("mouse receive: {e}"))?;
        if resp[1] != STATUS_OK {
            return Err(format!(
                "mouse rejected command {:02x}/{:02x}: status 0x{:02x}",
                rep[7], rep[8], resp[1]
            ));
        }
        Ok(resp)
    }

    pub fn status(&self) -> Result<MouseStatus, String> {
        let fw = self.transact(&Self::report(0x00, 0x81, 0x02, &[]))?;
        let logo = self.transact(&Self::report(0x0F, 0x84, 0x03, &[VARSTORE, LED_LOGO, 0]))?;
        let scroll = self.transact(&Self::report(0x0F, 0x84, 0x03, &[VARSTORE, LED_SCROLL, 0]))?;
        Ok(MouseStatus {
            firmware: format!("v{}.{}", fw[9], fw[10]),
            logo_brightness: logo[11],
            scroll_brightness: scroll[11],
        })
    }

    /// Static colour + brightness on both LEDs, stored on the mouse (VARSTORE).
    pub fn set_static(&self, rgb: [u8; 3], brightness: u8) -> Result<(), String> {
        for led in [LED_LOGO, LED_SCROLL] {
            self.transact(&Self::report(
                0x0F,
                0x02,
                0x09,
                &[VARSTORE, led, 0x01, 0, 0, 0x01, rgb[0], rgb[1], rgb[2]],
            ))?;
            self.transact(&Self::report(0x0F, 0x04, 0x03, &[VARSTORE, led, brightness]))?;
        }
        Ok(())
    }

    pub fn set_off(&self) -> Result<(), String> {
        for led in [LED_LOGO, LED_SCROLL] {
            self.transact(&Self::report(0x0F, 0x02, 0x06, &[VARSTORE, led, 0x00, 0, 0, 0]))?;
        }
        Ok(())
    }
}
