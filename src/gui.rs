use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use serde::{Deserialize, Serialize};
use tao::{
    dpi::{LogicalSize, PhysicalPosition},
    event::{Event, StartCause, WindowEvent},
    event_loop::{ControlFlow, EventLoopBuilder},
    window::WindowBuilder,
};
use tray_icon::{
    Icon, TrayIconBuilder,
    menu::{Menu, MenuItem, PredefinedMenuItem},
};
use wry::WebViewBuilder;

use crate::config::AppConfig;
use crate::cursor;
use crate::fix::{self, FixStatus};
use crate::process;

const HTML_CONTENT: &str = include_str!("ui.html");

#[derive(Debug, Clone, Serialize, Deserialize)]
struct IpcMessage {
    action: String,
    #[serde(default)]
    config: Option<AppConfigPayload>,
    #[serde(default)]
    action_type: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct AppConfigPayload {
    target_process: String,
    scheme: String,
    size: u32,
    auto_switch: bool,
}

#[derive(Debug, Clone, Serialize)]
struct UiState {
    config: AppConfig,
    schemes: Vec<String>,
    status: String,
    #[serde(rename = "fixStatus")]
    fix_status: String,
}

#[derive(Debug, Clone, Serialize)]
struct FixConfirmInfo {
    #[serde(rename = "isDisabling")]
    is_disabling: bool,
    message: String,
    action: String,
}

enum UserEvent {
    PollTimer,
}

static IS_TARGET_RUNNING: AtomicBool = AtomicBool::new(false);
static HAS_SAVED_BACKUP: AtomicBool = AtomicBool::new(false);

fn load_app_icon() -> Icon {
    let width = 32;
    let height = 32;
    let mut rgba = Vec::with_capacity((width * height * 4) as usize);
    for y in 0..height {
        for x in 0..width {
            let dx = (x as f32 - 16.0).abs();
            let dy = (y as f32 - 16.0).abs();
            if dx + dy < 13.0 {
                rgba.extend_from_slice(&[6, 182, 212, 255]); // Cyan #06b6d4
            } else {
                rgba.extend_from_slice(&[0, 0, 0, 0]);
            }
        }
    }
    Icon::from_rgba(rgba, width, height).unwrap()
}

pub fn run_gui() {
    let event_loop = EventLoopBuilder::<UserEvent>::with_user_event().build();

    let window = WindowBuilder::new()
        .with_title("AK Cursor Switcher")
        .with_inner_size(LogicalSize::new(550.0, 540.0))
        .with_resizable(false)
        .build(&event_loop)
        .expect("Failed to create tao window");

    // Center window on primary monitor
    if let Some(monitor) = window
        .primary_monitor()
        .or_else(|| window.available_monitors().next())
    {
        let screen_size = monitor.size();
        let window_size = window.outer_size();
        let x = screen_size.width.saturating_sub(window_size.width) / 2;
        let y = screen_size.height.saturating_sub(window_size.height) / 2;
        window.set_outer_position(PhysicalPosition::new(x as i32, y as i32));
    }

    let window = Arc::new(window);

    // Build system tray menu
    let tray_menu = Menu::new();
    let item_open = MenuItem::new("Open Window", true, None);
    let item_exit = MenuItem::new("Exit", true, None);
    let _ = tray_menu.append(&item_open);
    let _ = tray_menu.append(&PredefinedMenuItem::separator());
    let _ = tray_menu.append(&item_exit);

    let mut tray_icon = TrayIconBuilder::new()
        .with_menu(Box::new(tray_menu))
        .with_tooltip("AK Cursor Switcher")
        .with_icon(load_app_icon())
        .build()
        .ok();

    let open_id = item_open.id().clone();
    let exit_id = item_exit.id().clone();

    // Background timer thread for 1s process polling
    let proxy = event_loop.create_proxy();
    std::thread::spawn(move || {
        loop {
            std::thread::sleep(Duration::from_millis(1000));
            if proxy.send_event(UserEvent::PollTimer).is_err() {
                break;
            }
        }
    });

    let current_status = Arc::new(Mutex::new(String::from("Monitoring (Idle)")));

    // Initial backup on launch
    cursor::save_current();
    HAS_SAVED_BACKUP.store(true, Ordering::SeqCst);

    let webview_holder = Arc::new(Mutex::new(None::<wry::WebView>));

    let window_clone = Arc::clone(&window);
    let status_clone = Arc::clone(&current_status);
    let webview_ref = Arc::clone(&webview_holder);

    let ipc_handler = move |req: wry::http::Request<String>| {
        if let Ok(msg) = serde_json::from_str::<IpcMessage>(req.body()) {
            match msg.action.as_str() {
                "init" => {
                    let cfg = AppConfig::load();
                    let schemes = cursor::get_available_schemes();
                    let fix_stat = if let Some(path) = fix::locate_asset_file(Some(&cfg.game_path))
                    {
                        match fix::get_status_of_file(&path) {
                            FixStatus::ActiveSoftwareCursor => "ActiveSoftwareCursor",
                            FixStatus::FixedDisabled => "FixedDisabled",
                            FixStatus::NotFound => "NotFound",
                        }
                    } else {
                        "NotFound"
                    };

                    let st = status_clone.lock().unwrap().clone();
                    let state = UiState {
                        config: cfg,
                        schemes,
                        status: st,
                        fix_status: fix_stat.to_string(),
                    };

                    if let Ok(json) = serde_json::to_string(&state) {
                        if let Some(wv) = webview_ref.lock().unwrap().as_ref() {
                            let _ = wv.evaluate_script(&format!("window.updateState({});", json));
                        }
                    }
                }
                "save_config" => {
                    if let Some(payload) = msg.config {
                        let mut cfg = AppConfig::load();
                        cfg.target_process = payload.target_process;
                        cfg.scheme = payload.scheme;
                        cfg.size = payload.size;
                        cfg.auto_switch = payload.auto_switch;
                        cfg.save();
                    }
                }
                "apply_now" => {
                    let cfg = AppConfig::load();
                    if !cfg.scheme.is_empty() {
                        cursor::apply_scheme_and_size(&cfg.scheme, cfg.size);
                    } else {
                        cursor::set_cursor_size(cfg.size);
                    }
                    *status_clone.lock().unwrap() =
                        format!("Applied {} (size {})", cfg.scheme, cfg.size);

                    if let Some(wv) = webview_ref.lock().unwrap().as_ref() {
                        let _ = wv.evaluate_script(&format!(
                            "window.updateState({{ status: '{}' }});",
                            status_clone.lock().unwrap()
                        ));
                    }
                }
                "restore_defaults" => {
                    cursor::restore_scheme();
                    *status_clone.lock().unwrap() = String::from("Restored Windows defaults");
                    if let Some(wv) = webview_ref.lock().unwrap().as_ref() {
                        let _ = wv.evaluate_script(&format!(
                            "window.updateState({{ status: '{}' }});",
                            status_clone.lock().unwrap()
                        ));
                    }
                }
                "browse_exe" => {
                    if let Some(file) = rfd::FileDialog::new()
                        .add_filter("Arknights", &["exe", "bin"])
                        .set_title("Select Arknights Executable")
                        .pick_file()
                    {
                        let mut cfg = AppConfig::load();
                        let path_str = file.to_string_lossy().to_string();
                        if let Some(name) = file.file_name().and_then(|n| n.to_str()) {
                            if name.ends_with(".exe") {
                                cfg.target_process = name.to_string();
                            }
                        }
                        cfg.game_path = path_str;
                        cfg.save();

                        // Refresh state
                        let schemes = cursor::get_available_schemes();
                        let fix_stat =
                            if let Some(path) = fix::locate_asset_file(Some(&cfg.game_path)) {
                                match fix::get_status_of_file(&path) {
                                    FixStatus::ActiveSoftwareCursor => "ActiveSoftwareCursor",
                                    FixStatus::FixedDisabled => "FixedDisabled",
                                    FixStatus::NotFound => "NotFound",
                                }
                            } else {
                                "NotFound"
                            };

                        let state = UiState {
                            config: cfg,
                            schemes,
                            status: String::from("Game located."),
                            fix_status: fix_stat.to_string(),
                        };

                        if let Ok(json) = serde_json::to_string(&state) {
                            if let Some(wv) = webview_ref.lock().unwrap().as_ref() {
                                let _ =
                                    wv.evaluate_script(&format!("window.updateState({});", json));
                            }
                        }
                    }
                }
                "hide_to_tray" => {
                    window_clone.set_visible(false);
                }
                "request_fix_info" => {
                    let cfg = AppConfig::load();
                    if let Some(path) = fix::locate_asset_file(Some(&cfg.game_path)) {
                        let status = fix::get_status_of_file(&path);
                        let parent_dir = path
                            .parent()
                            .map(|p| p.display().to_string())
                            .unwrap_or_default();
                        match status {
                            FixStatus::ActiveSoftwareCursor => {
                                let src_name = fix::ASSET_FILENAME;
                                let dst_name = fix::ASSET_FILENAME_DISABLED;
                                let msg = format!(
                                    "Disables PRTS cursor bundle to use native Windows hardware cursor.<div class='path-box'><div class='path-label'>Folder</div><div class='path-dir'>{}</div><div class='path-label' style='margin-top:4px;'>Rename</div><div class='path-change'><span class='path-file-old'>{}</span> <span class='path-arrow'>&rarr;</span> <span class='path-file-new'>{}</span></div></div>",
                                    parent_dir, src_name, dst_name
                                );
                                let info = FixConfirmInfo {
                                    is_disabling: true,
                                    message: msg,
                                    action: "disable".to_string(),
                                };
                                if let Ok(json) = serde_json::to_string(&info) {
                                    if let Some(wv) = webview_ref.lock().unwrap().as_ref() {
                                        let _ = wv.evaluate_script(&format!(
                                            "window.showFixConfirmation({});",
                                            json
                                        ));
                                    }
                                }
                            }
                            FixStatus::FixedDisabled => {
                                let src_name = fix::ASSET_FILENAME_DISABLED;
                                let dst_name = fix::ASSET_FILENAME;
                                let msg = format!(
                                    "Enables PRTS in-game cursor bundle.<div class='path-box'><div class='path-label'>Folder</div><div class='path-dir'>{}</div><div class='path-label' style='margin-top:4px;'>Rename</div><div class='path-change'><span class='path-file-old'>{}</span> <span class='path-arrow'>&rarr;</span> <span class='path-file-new'>{}</span></div></div>",
                                    parent_dir, src_name, dst_name
                                );
                                let info = FixConfirmInfo {
                                    is_disabling: false,
                                    message: msg,
                                    action: "restore".to_string(),
                                };
                                if let Ok(json) = serde_json::to_string(&info) {
                                    if let Some(wv) = webview_ref.lock().unwrap().as_ref() {
                                        let _ = wv.evaluate_script(&format!(
                                            "window.showFixConfirmation({});",
                                            json
                                        ));
                                    }
                                }
                            }
                            FixStatus::NotFound => {}
                        }
                    }
                }
                "execute_fix" => {
                    let mut cfg = AppConfig::load();
                    if let Some(path) = fix::locate_asset_file(Some(&cfg.game_path)) {
                        if let Ok((new_status, new_path)) = fix::toggle_fix(&path) {
                            if let Some(parent) = new_path.parent() {
                                cfg.game_path = parent.to_string_lossy().to_string();
                                cfg.save();
                            }
                            let fix_stat_str = match new_status {
                                FixStatus::ActiveSoftwareCursor => "ActiveSoftwareCursor",
                                FixStatus::FixedDisabled => "FixedDisabled",
                                FixStatus::NotFound => "NotFound",
                            };
                            let status_text = match new_status {
                                FixStatus::FixedDisabled => {
                                    "PRTS cursor disabled (Hardware cursor active)"
                                }
                                FixStatus::ActiveSoftwareCursor => {
                                    "PRTS cursor enabled (.bin active)"
                                }
                                FixStatus::NotFound => "Status: Updated",
                            };

                            if let Some(wv) = webview_ref.lock().unwrap().as_ref() {
                                let _ = wv.evaluate_script(&format!(
                                    "window.updateState({{ fixStatus: '{}', status: '{}' }});",
                                    fix_stat_str, status_text
                                ));
                            }
                        }
                    }
                }
                _ => {}
            }
        }
    };

    let webview = WebViewBuilder::new()
        .with_html(HTML_CONTENT)
        .with_ipc_handler(ipc_handler)
        .build(&window)
        .expect("Failed to build wry webview");

    *webview_holder.lock().unwrap() = Some(webview);

    let status_loop = Arc::clone(&current_status);
    let webview_loop = Arc::clone(&webview_holder);
    let window_loop = Arc::clone(&window);

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;

        if let Ok(menu_event) = tray_icon::menu::MenuEvent::receiver().try_recv() {
            if menu_event.id == open_id {
                window_loop.set_visible(true);
                window_loop.set_focus();
            } else if menu_event.id == exit_id {
                *control_flow = ControlFlow::Exit;
            }
        }

        match event {
            Event::NewEvents(StartCause::Init) => {}
            Event::UserEvent(UserEvent::PollTimer) => {
                let cfg = AppConfig::load();
                if cfg.auto_switch && !cfg.target_process.trim().is_empty() {
                    let running = process::is_running(cfg.target_process.trim());
                    let was_running = IS_TARGET_RUNNING.load(Ordering::SeqCst);

                    if running && !was_running {
                        if !HAS_SAVED_BACKUP.load(Ordering::SeqCst) {
                            cursor::save_current();
                            HAS_SAVED_BACKUP.store(true, Ordering::SeqCst);
                        }
                        if !cfg.scheme.is_empty() {
                            cursor::apply_scheme_and_size(&cfg.scheme, cfg.size);
                        } else {
                            cursor::set_cursor_size(cfg.size);
                        }
                        *status_loop.lock().unwrap() =
                            format!("Active ({} running)", cfg.target_process);
                        IS_TARGET_RUNNING.store(true, Ordering::SeqCst);

                        if let Some(wv) = webview_loop.lock().unwrap().as_ref() {
                            let _ = wv.evaluate_script(&format!(
                                "window.updateState({{ status: '{}' }});",
                                status_loop.lock().unwrap()
                            ));
                        }
                    } else if !running && was_running {
                        cursor::restore_scheme();
                        *status_loop.lock().unwrap() =
                            format!("Idle (Watching {})", cfg.target_process);
                        IS_TARGET_RUNNING.store(false, Ordering::SeqCst);

                        if let Some(wv) = webview_loop.lock().unwrap().as_ref() {
                            let _ = wv.evaluate_script(&format!(
                                "window.updateState({{ status: '{}' }});",
                                status_loop.lock().unwrap()
                            ));
                        }
                    }
                }
            }
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => {
                *control_flow = ControlFlow::Exit;
            }
            Event::LoopDestroyed => {
                drop(tray_icon.take());
            }
            _ => {}
        }
    });
}
