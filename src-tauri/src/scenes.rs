//! Scenes + last-applied state, persisted as one JSON file in the app data dir.
//! A Scene sets both devices at once; `None` for a device means "leave it alone",
//! `Some(Light { on: false, .. })` means off.

use serde::{Deserialize, Serialize};
use std::{fs, path::PathBuf, sync::Mutex};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Light {
    pub on: bool,
    pub rgb: [u8; 3],
    /// 0–100, the UI's scale; each driver maps it to its own (0–9 or 0–255).
    pub brightness: u8,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Scene {
    pub id: String,
    pub name: String,
    pub keyboard: Option<Light>,
    pub mouse: Option<Light>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct Store {
    pub scenes: Vec<Scene>,
    pub keyboard: Option<Light>,
    pub mouse: Option<Light>,
    /// Set once the first installed run has registered Launch-at-startup, so a
    /// user who later turns it off is not re-enrolled on every launch.
    #[serde(default)]
    pub autostart_seeded: bool,
}

pub struct Scenes {
    path: PathBuf,
    pub store: Mutex<Store>,
}

fn default_scenes() -> Vec<Scene> {
    let cool = Light { on: true, rgb: [220, 235, 255], brightness: 40 };
    let warm = Light { on: true, rgb: [255, 214, 170], brightness: 40 };
    let off = Light { on: false, rgb: [0, 0, 0], brightness: 0 };
    vec![
        Scene { id: "cool-white".into(), name: "Cool White 40%".into(), keyboard: Some(cool.clone()), mouse: Some(cool) },
        Scene { id: "warm-white".into(), name: "Warm White 40%".into(), keyboard: Some(warm.clone()), mouse: Some(warm) },
        Scene { id: "off".into(), name: "Lights Off".into(), keyboard: Some(off.clone()), mouse: Some(off) },
    ]
}

impl Scenes {
    pub fn load(dir: PathBuf) -> Scenes {
        fs::create_dir_all(&dir).ok();
        let path = dir.join("deetsrgb.json");
        let store = fs::read_to_string(&path)
            .ok()
            .and_then(|s| serde_json::from_str::<Store>(&s).ok())
            .unwrap_or_else(|| Store { scenes: default_scenes(), ..Default::default() });
        Scenes { path, store: Mutex::new(store) }
    }

    pub fn save(&self) -> Result<(), String> {
        let store = self.store.lock().unwrap();
        let json = serde_json::to_string_pretty(&*store).map_err(|e| e.to_string())?;
        fs::write(&self.path, json).map_err(|e| format!("save {}: {e}", self.path.display()))
    }
}
