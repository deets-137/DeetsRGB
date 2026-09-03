// The Rust boundary. Every hardware call goes through here; the UI never sees
// a report byte. Types mirror src-tauri/src/scenes.rs one-to-one.
import { invoke } from "@tauri-apps/api/core";

export type RGB = [number, number, number];

export interface Light {
  on: boolean;
  rgb: RGB;
  /** 0–100; Rust maps it onto each device's own scale (keyboard 0–9, mouse 0–255). */
  brightness: number;
}

export interface Scene {
  id: string;
  name: string;
  keyboard: Light | null;
  mouse: Light | null;
}

export interface KeyboardStatus {
  effect: number;
  effect_name: string;
  brightness: number; // 0–9
  rgb: RGB;
}

export interface MouseStatus {
  firmware: string;
  logo_brightness: number; // 0–255
  scroll_brightness: number;
}

export interface DevicesStatus {
  keyboard: KeyboardStatus | null;
  keyboard_error: string | null;
  mouse: MouseStatus | null;
  mouse_error: string | null;
  last_keyboard: Light | null;
  last_mouse: Light | null;
}

export type DeviceId = "keyboard" | "mouse";

export const devicesStatus = () => invoke<DevicesStatus>("devices_status");
export const setDevice = (device: DeviceId, light: Light) =>
  invoke<void>(device === "keyboard" ? "keyboard_set" : "mouse_set", { light });
export const scenesList = () => invoke<Scene[]>("scenes_list");
export const sceneSave = (scene: Scene) => invoke<Scene[]>("scene_save", { scene });
export const sceneDelete = (id: string) => invoke<Scene[]>("scene_delete", { id });
export const sceneApply = (id: string) => invoke<void>("scene_apply", { id });
export const autostartGet = () => invoke<boolean>("autostart_get");
export const autostartSet = (on: boolean) => invoke<boolean>("autostart_set", { on });
export const panelHide = () => invoke<void>("panel_hide");
export const appQuit = () => invoke<void>("app_quit");
