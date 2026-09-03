mod razer;
mod scenes;
mod sinowealth;

use hidapi::HidApi;
use scenes::{Light, Scene, Scenes};
use serde::Serialize;
use std::{
    sync::Mutex,
    time::{Duration, Instant},
};
use tauri::{
    menu::{MenuBuilder, MenuItemBuilder},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, Manager, PhysicalPosition, State, WindowEvent,
};

/// The HID context is opened once; devices are re-found on every call so a
/// re-plugged keyboard or mouse just works without restarting the tray.
struct Hid(Mutex<HidApi>);

/// The panel hides when it loses focus. A tray click that *caused* that blur
/// would otherwise re-show it instantly — remember when we last hid.
struct HiddenAt(Mutex<Option<Instant>>);

#[derive(Serialize)]
struct DevicesStatus {
    keyboard: Option<sinowealth::KeyboardStatus>,
    keyboard_error: Option<String>,
    mouse: Option<razer::MouseStatus>,
    mouse_error: Option<String>,
    /// Last state DeetsRGB itself applied (the mouse can't report its colour back).
    last_keyboard: Option<Light>,
    last_mouse: Option<Light>,
}

fn kb_brightness(pct: u8) -> u8 {
    // 0–100 → the firmware's 0–9, rounded (40% → 4).
    ((pct.min(100) as u16 * 9 + 50) / 100) as u8
}

fn mouse_brightness(pct: u8) -> u8 {
    ((pct.min(100) as u16 * 255 + 50) / 100) as u8
}

fn with_keyboard<T>(hid: &Hid, f: impl FnOnce(&sinowealth::Keyboard) -> Result<T, String>) -> Result<T, String> {
    let mut api = hid.0.lock().unwrap();
    api.refresh_devices().map_err(|e| e.to_string())?;
    let kb = sinowealth::Keyboard::open(&api).ok_or("keyboard not found (258A:010C, vendor collection)")?;
    f(&kb)
}

fn with_mouse<T>(hid: &Hid, f: impl FnOnce(&razer::Mouse) -> Result<T, String>) -> Result<T, String> {
    let mut api = hid.0.lock().unwrap();
    api.refresh_devices().map_err(|e| e.to_string())?;
    let m = razer::Mouse::open(&api).ok_or("mouse not found (1532:005C, interface 0)")?;
    f(&m)
}

fn apply_keyboard(hid: &Hid, scenes: &Scenes, light: &Light) -> Result<(), String> {
    with_keyboard(hid, |kb| {
        if light.on {
            kb.set_solid(light.rgb, kb_brightness(light.brightness))
        } else {
            kb.set_off()
        }
    })?;
    scenes.store.lock().unwrap().keyboard = Some(light.clone());
    scenes.save()
}

fn apply_mouse(hid: &Hid, scenes: &Scenes, light: &Light) -> Result<(), String> {
    with_mouse(hid, |m| {
        if light.on {
            m.set_static(light.rgb, mouse_brightness(light.brightness))
        } else {
            m.set_off()
        }
    })?;
    scenes.store.lock().unwrap().mouse = Some(light.clone());
    scenes.save()
}

fn apply_scene(hid: &Hid, scenes: &Scenes, id: &str) -> Result<(), String> {
    let scene = scenes
        .store
        .lock()
        .unwrap()
        .scenes
        .iter()
        .find(|s| s.id == id)
        .cloned()
        .ok_or("no such scene")?;
    let mut errors = vec![];
    if let Some(l) = &scene.keyboard {
        if let Err(e) = apply_keyboard(hid, scenes, l) {
            errors.push(e);
        }
    }
    if let Some(l) = &scene.mouse {
        if let Err(e) = apply_mouse(hid, scenes, l) {
            errors.push(e);
        }
    }
    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors.join("; "))
    }
}

// ── commands ──────────────────────────────────────────────────────────────

#[tauri::command]
fn devices_status(hid: State<Hid>, scenes: State<Scenes>) -> DevicesStatus {
    let (kb, kb_err) = match with_keyboard(&hid, |k| k.status()) {
        Ok(s) => (Some(s), None),
        Err(e) => (None, Some(e)),
    };
    let (m, m_err) = match with_mouse(&hid, |m| m.status()) {
        Ok(s) => (Some(s), None),
        Err(e) => (None, Some(e)),
    };
    let store = scenes.store.lock().unwrap();
    DevicesStatus {
        keyboard: kb,
        keyboard_error: kb_err,
        mouse: m,
        mouse_error: m_err,
        last_keyboard: store.keyboard.clone(),
        last_mouse: store.mouse.clone(),
    }
}

#[tauri::command]
fn keyboard_set(light: Light, hid: State<Hid>, scenes: State<Scenes>) -> Result<(), String> {
    apply_keyboard(&hid, &scenes, &light)
}

#[tauri::command]
fn mouse_set(light: Light, hid: State<Hid>, scenes: State<Scenes>) -> Result<(), String> {
    apply_mouse(&hid, &scenes, &light)
}

#[tauri::command]
fn scenes_list(scenes: State<Scenes>) -> Vec<Scene> {
    scenes.store.lock().unwrap().scenes.clone()
}

#[tauri::command]
fn scene_save(app: AppHandle, scene: Scene, scenes: State<Scenes>) -> Result<Vec<Scene>, String> {
    {
        let mut store = scenes.store.lock().unwrap();
        match store.scenes.iter_mut().find(|s| s.id == scene.id) {
            Some(existing) => *existing = scene,
            None => store.scenes.push(scene),
        }
    }
    scenes.save()?;
    rebuild_tray_menu(&app);
    Ok(scenes.store.lock().unwrap().scenes.clone())
}

#[tauri::command]
fn scene_delete(app: AppHandle, id: String, scenes: State<Scenes>) -> Result<Vec<Scene>, String> {
    scenes.store.lock().unwrap().scenes.retain(|s| s.id != id);
    scenes.save()?;
    rebuild_tray_menu(&app);
    Ok(scenes.store.lock().unwrap().scenes.clone())
}

#[tauri::command]
fn scene_apply(id: String, hid: State<Hid>, scenes: State<Scenes>) -> Result<(), String> {
    apply_scene(&hid, &scenes, &id)
}

// ── launch at startup (HKCU Run key, via reg.exe — no crate needed) ────────

const RUN_KEY: &str = r"HKCU\Software\Microsoft\Windows\CurrentVersion\Run";
const RUN_VALUE: &str = "DeetsRGB";

fn reg(args: &[&str]) -> Result<String, String> {
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    let out = std::process::Command::new("reg")
        .args(args)
        .creation_flags(CREATE_NO_WINDOW)
        .output()
        .map_err(|e| format!("reg.exe: {e}"))?;
    Ok(String::from_utf8_lossy(&out.stdout).into_owned())
}

fn autostart_enabled() -> bool {
    reg(&["query", RUN_KEY, "/v", RUN_VALUE]).map(|s| s.contains(RUN_VALUE)).unwrap_or(false)
}

fn autostart_write(on: bool) -> Result<(), String> {
    if on {
        let exe = std::env::current_exe().map_err(|e| e.to_string())?;
        let cmd = format!("\"{}\"", exe.display());
        reg(&["add", RUN_KEY, "/v", RUN_VALUE, "/t", "REG_SZ", "/d", &cmd, "/f"])?;
    } else {
        reg(&["delete", RUN_KEY, "/v", RUN_VALUE, "/f"])?;
    }
    Ok(())
}

#[tauri::command]
fn autostart_get() -> bool {
    autostart_enabled()
}

#[tauri::command]
fn autostart_set(on: bool) -> Result<bool, String> {
    autostart_write(on)?;
    Ok(autostart_enabled())
}

#[tauri::command]
fn panel_hide(app: AppHandle) {
    hide_panel(&app);
}

#[tauri::command]
fn app_quit(app: AppHandle) {
    app.exit(0);
}

// ── tray + panel ──────────────────────────────────────────────────────────

fn hide_panel(app: &AppHandle) {
    if let Some(w) = app.get_webview_window("main") {
        w.hide().ok();
        *app.state::<HiddenAt>().0.lock().unwrap() = Some(Instant::now());
    }
}

/// Show the panel with its bottom-right corner at the tray click, kept on-screen.
fn show_panel(app: &AppHandle, at: Option<PhysicalPosition<f64>>) {
    let Some(w) = app.get_webview_window("main") else { return };
    if let Some(p) = at {
        let size = w.outer_size().unwrap_or_default();
        let (mut x, mut y) = (p.x - size.width as f64, p.y - size.height as f64 - 8.0);
        if let Ok(Some(mon)) = app.monitor_from_point(p.x, p.y) {
            let (mp, ms) = (mon.position(), mon.size());
            x = x.max(mp.x as f64).min((mp.x + ms.width as i32) as f64 - size.width as f64);
            y = y.max(mp.y as f64);
        }
        w.set_position(PhysicalPosition::new(x, y)).ok();
    }
    w.show().ok();
    w.set_focus().ok();
}

fn toggle_panel(app: &AppHandle, at: PhysicalPosition<f64>) {
    let visible = app
        .get_webview_window("main")
        .and_then(|w| w.is_visible().ok())
        .unwrap_or(false);
    if visible {
        hide_panel(app);
        return;
    }
    // The click that just blurred (and hid) the panel must not re-open it.
    let just_hid = app
        .state::<HiddenAt>()
        .0
        .lock()
        .unwrap()
        .map(|t| t.elapsed() < Duration::from_millis(300))
        .unwrap_or(false);
    if !just_hid {
        show_panel(app, Some(at));
    }
}

fn rebuild_tray_menu(app: &AppHandle) {
    let Some(tray) = app.tray_by_id("main") else { return };
    let mut menu = MenuBuilder::new(app);
    for s in app.state::<Scenes>().store.lock().unwrap().scenes.iter() {
        if let Ok(item) = MenuItemBuilder::with_id(format!("scene:{}", s.id), &s.name).build(app) {
            menu = menu.item(&item);
        }
    }
    let open = MenuItemBuilder::with_id("open", "Open DeetsRGB").build(app);
    let quit = MenuItemBuilder::with_id("quit", "Quit DeetsRGB").build(app);
    if let (Ok(open), Ok(quit)) = (open, quit) {
        if let Ok(m) = menu.separator().item(&open).item(&quit).build() {
            tray.set_menu(Some(m)).ok();
        }
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(HiddenAt(Mutex::new(None)))
        .setup(|app| {
            let hid = HidApi::new().expect("hidapi init");
            app.manage(Hid(Mutex::new(hid)));

            let dir = app.path().app_data_dir().expect("app data dir");
            app.manage(Scenes::load(dir));

            // First run of an INSTALLED build enrols in Launch-at-startup once; the
            // title-menu toggle owns it from then on. Dev builds never touch the key.
            #[cfg(not(debug_assertions))]
            {
                let scenes = app.state::<Scenes>();
                let seeded = scenes.store.lock().unwrap().autostart_seeded;
                if !seeded {
                    if let Err(e) = autostart_write(true) {
                        eprintln!("[autostart] {e}");
                    }
                    scenes.store.lock().unwrap().autostart_seeded = true;
                    scenes.save().ok();
                }
            }

            TrayIconBuilder::with_id("main")
                .icon(app.default_window_icon().expect("window icon").clone())
                .tooltip("DeetsRGB")
                .show_menu_on_left_click(false)
                .on_menu_event(|app, ev| {
                    let id = ev.id().as_ref();
                    match id {
                        "quit" => app.exit(0),
                        "open" => show_panel(app, None),
                        _ => {
                            if let Some(scene) = id.strip_prefix("scene:") {
                                if let Err(e) = apply_scene(&app.state::<Hid>(), &app.state::<Scenes>(), scene) {
                                    eprintln!("[scene] {e}");
                                }
                            }
                        }
                    }
                })
                .on_tray_icon_event(|tray, ev| {
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        position,
                        ..
                    } = ev
                    {
                        toggle_panel(tray.app_handle(), position);
                    }
                })
                .build(app)?;
            rebuild_tray_menu(app.handle());

            #[cfg(debug_assertions)]
            if let Some(win) = app.get_webview_window("main") {
                win.open_devtools();
            }
            Ok(())
        })
        .on_window_event(|win, ev| match ev {
            // Closing the panel hides it; the tray menu owns Quit.
            WindowEvent::CloseRequested { api, .. } => {
                api.prevent_close();
                hide_panel(win.app_handle());
            }
            // A tray popover: clicking anywhere else dismisses it. Devtools steal focus in
            // dev, so the popover behaviour is release-only.
            #[cfg(not(debug_assertions))]
            WindowEvent::Focused(false) => hide_panel(win.app_handle()),
            _ => {}
        })
        .invoke_handler(tauri::generate_handler![
            devices_status,
            keyboard_set,
            mouse_set,
            scenes_list,
            scene_save,
            scene_delete,
            scene_apply,
            autostart_get,
            autostart_set,
            panel_hide,
            app_quit,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
