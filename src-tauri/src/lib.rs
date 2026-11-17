use std::sync::atomic::{AtomicBool, Ordering};
use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    Emitter, Manager, PhysicalPosition, WebviewUrl, WebviewWindowBuilder,
};
use tauri_plugin_autostart::ManagerExt;
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutState};
use tauri_plugin_notification::NotificationExt;

static IS_MUTED: AtomicBool = AtomicBool::new(false);

#[tauri::command]
fn send_native_notification(
    app: tauri::AppHandle,
    title: String,
    body: String,
) -> Result<(), String> {
    app.notification()
        .builder()
        .title(title)
        .body(body)
        .show()
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn is_autostart_enabled(app: tauri::AppHandle) -> Result<bool, String> {
    app.autolaunch().is_enabled().map_err(|e| e.to_string())
}

#[tauri::command]
fn set_autostart(app: tauri::AppHandle, enable: bool) -> Result<bool, String> {
    if enable {
        app.autolaunch().enable().map_err(|e| e.to_string())?;
    } else {
        app.autolaunch().disable().map_err(|e| e.to_string())?;
    }
    app.autolaunch().is_enabled().map_err(|e| e.to_string())
}

#[tauri::command]
fn hide_main_window(app: tauri::AppHandle) -> Result<(), String> {
    if let Some(win) = app.get_webview_window("main") {
        win.hide().map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
fn show_main_window(app: tauri::AppHandle) -> Result<(), String> {
    if let Some(win) = app.get_webview_window("main") {
        win.show().map_err(|e| e.to_string())?;
        win.unminimize().map_err(|e| e.to_string())?;
        win.set_focus().map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
fn minimize_window(app: tauri::AppHandle) -> Result<(), String> {
    if let Some(win) = app.get_webview_window("main") {
        win.minimize().map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
fn toggle_maximize_window(app: tauri::AppHandle) -> Result<bool, String> {
    if let Some(win) = app.get_webview_window("main") {
        let is_max = win.is_maximized().unwrap_or(false);
        if is_max {
            win.unmaximize().map_err(|e| e.to_string())?;
            return Ok(false);
        } else {
            win.maximize().map_err(|e| e.to_string())?;
            return Ok(true);
        }
    }
    Ok(false)
}

#[tauri::command]
fn toggle_main_window(app: tauri::AppHandle) -> Result<bool, String> {
    if let Some(win) = app.get_webview_window("main") {
        if win.is_visible().unwrap_or(false) {
            win.hide().map_err(|e| e.to_string())?;
            return Ok(false);
        } else {
            win.show().map_err(|e| e.to_string())?;
            win.unminimize().map_err(|e| e.to_string())?;
            win.set_focus().map_err(|e| e.to_string())?;
            return Ok(true);
        }
    }
    Ok(false)
}

#[tauri::command]
fn toggle_mute(app: tauri::AppHandle) -> Result<bool, String> {
    let current = IS_MUTED.load(Ordering::SeqCst);
    let new_val = !current;
    IS_MUTED.store(new_val, Ordering::SeqCst);
    let _ = app.emit("audio-mute-changed", new_val);
    Ok(new_val)
}

#[tauri::command]
fn get_mute_status() -> bool {
    IS_MUTED.load(Ordering::SeqCst)
}

#[tauri::command]
fn drag_widget_window(win: tauri::WebviewWindow) -> Result<(), String> {
    win.start_dragging().map_err(|e| e.to_string())
}

fn log_widget_event(msg: &str) {
    use std::io::Write;
    if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open("D:\\Soft\\Miqat\\widget_runtime.log") {
        let _ = writeln!(f, "[{}] {}", chrono_or_simple_time(), msg);
    }
}

fn chrono_or_simple_time() -> String {
    use std::time::SystemTime;
    let now = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap_or_default();
    format!("{}.{}", now.as_secs(), now.subsec_millis())
}

#[cfg(target_os = "windows")]
fn send_to_desktop_bottom(win: &tauri::WebviewWindow) {
    let _ = win.set_always_on_top(false);
    if let Ok(hwnd) = win.hwnd() {
        use windows_sys::Win32::UI::WindowsAndMessaging::{
            SetWindowPos, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE,
        };
        unsafe {
            SetWindowPos(
                hwnd.0 as _,
                1 as _, // HWND_BOTTOM
                0, 0, 0, 0,
                SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
            );
        }
    }
}

#[cfg(not(target_os = "windows"))]
fn send_to_desktop_bottom(win: &tauri::WebviewWindow) {
    let _ = win.set_always_on_top(false);
}

#[tauri::command]
async fn open_widget_window(
    app: tauri::AppHandle,
    widget_id: String,
    x: Option<i32>,
    y: Option<i32>,
    width: Option<f64>,
    height: Option<f64>,
    always_on_top: Option<bool>,
) -> Result<(), String> {
    let label = format!("widget_{}", widget_id);
    let on_top = always_on_top.unwrap_or(false);
    log_widget_event(&format!("open_widget_window: id={} x={:?} y={:?} w={:?} h={:?} on_top={}", widget_id, x, y, width, height, on_top));

    // If window already exists, show it, unminimize it, and update position
    if let Some(win) = app.get_webview_window(&label) {
        log_widget_event(&format!("open_widget_window: window {} exists, restoring", label));
        let _ = win.unminimize();
        let _ = win.show();
        if on_top {
            let _ = win.set_always_on_top(true);
            let _ = win.set_focus();
        } else {
            send_to_desktop_bottom(&win);
            if let Some(main) = app.get_webview_window("main") {
                let _ = main.set_focus();
            }
        }
        if let (Some(px), Some(py)) = (x, y) {
            let _ = win.set_position(PhysicalPosition::new(px, py));
        }
        let _ = win.eval(&format!("try {{ window.__WIDGET_ID__ = '{}'; document.body.style.display = 'flex'; if (typeof initWidget === 'function') initWidget(); }} catch(e){{}}", widget_id));
        return Ok(());
    }

    log_widget_event(&format!("open_widget_window: building new window {}", label));
    let mut builder = WebviewWindowBuilder::new(
        &app,
        &label,
        WebviewUrl::App(format!("widget.html#{}", widget_id).into()),
    )
    .title(format!("ميقات - ودجت {}", widget_id))
    .decorations(false)
    .transparent(true)
    .skip_taskbar(true)
    .resizable(false)
    .always_on_top(on_top)
    .shadow(false);

    let (default_width, default_height) = match widget_id.as_str() {
        "miniPrayers" => (460.0, 300.0),
        "analogClock" => (340.0, 340.0),
        "clock" => (380.0, 220.0),
        "nextPrayer" => (380.0, 240.0),
        "workAlarm" => (380.0, 230.0),
        _ => (380.0, 240.0),
    };
    let w = width.unwrap_or(default_width);
    let h = height.unwrap_or(default_height);
    builder = builder.inner_size(w, h);

    let win = match builder.build() {
        Ok(w) => {
            log_widget_event(&format!("open_widget_window: built window {} SUCCESS", label));
            w
        }
        Err(e) => {
            log_widget_event(&format!("open_widget_window: build window {} FAILED: {:?}", label, e));
            return Err(e.to_string());
        }
    };

    if let (Some(px), Some(py)) = (x, y) {
        let _ = win.set_position(PhysicalPosition::new(px, py));
    }
    let _ = win.unminimize();
    let _ = win.show();
    if on_top {
        let _ = win.set_always_on_top(true);
        let _ = win.set_focus();
    } else {
        send_to_desktop_bottom(&win);
        if let Some(main) = app.get_webview_window("main") {
            let _ = main.set_focus();
        }
    }
    let _ = win.eval(&format!("try {{ window.__WIDGET_ID__ = '{}'; document.body.style.display = 'flex'; if (typeof initWidget === 'function') initWidget(); }} catch(e){{}}", widget_id));

    Ok(())
}

#[tauri::command]
async fn close_widget_window(app: tauri::AppHandle, widget_id: String) -> Result<(), String> {
    let label = format!("widget_{}", widget_id);
    if let Some(win) = app.get_webview_window(&label) {
        let _ = win.hide();
    }
    Ok(())
}

#[tauri::command]
async fn toggle_widget_window(
    app: tauri::AppHandle,
    widget_id: String,
    x: Option<i32>,
    y: Option<i32>,
    width: Option<f64>,
    height: Option<f64>,
    always_on_top: Option<bool>,
) -> Result<bool, String> {
    let label = format!("widget_{}", widget_id);
    let on_top = always_on_top.unwrap_or(false);
    if let Some(win) = app.get_webview_window(&label) {
        if win.is_visible().unwrap_or(false) {
            let _ = win.hide();
            return Ok(false);
        } else {
            let _ = win.unminimize();
            let _ = win.show();
            if let (Some(px), Some(py)) = (x, y) {
                let _ = win.set_position(PhysicalPosition::new(px, py));
            }
            if on_top {
                let _ = win.set_always_on_top(true);
                let _ = win.set_focus();
            } else {
                send_to_desktop_bottom(&win);
                if let Some(main) = app.get_webview_window("main") {
                    let _ = main.set_focus();
                }
            }
            return Ok(true);
        }
    } else {
        open_widget_window(app, widget_id, x, y, width, height, always_on_top).await?;
        return Ok(true);
    }
}

#[tauri::command]
fn get_widget_position(
    app: tauri::AppHandle,
    widget_id: String,
) -> Result<(i32, i32), String> {
    let label = format!("widget_{}", widget_id);
    if let Some(win) = app.get_webview_window(&label) {
        let pos = win.outer_position().map_err(|e| e.to_string())?;
        return Ok((pos.x, pos.y));
    }
    Err("Widget window not found".to_string())
}

#[tauri::command]
fn set_widget_position(
    app: tauri::AppHandle,
    widget_id: String,
    x: i32,
    y: i32,
) -> Result<(), String> {
    let label = format!("widget_{}", widget_id);
    if let Some(win) = app.get_webview_window(&label) {
        win.set_position(PhysicalPosition::new(x, y))
            .map_err(|e| e.to_string())?;
        return Ok(());
    }
    Err("Widget window not found".to_string())
}

#[tauri::command]
fn resize_widget_window(
    app: tauri::AppHandle,
    widget_id: String,
    width: f64,
    height: f64,
) -> Result<(), String> {
    let label = format!("widget_{}", widget_id);
    if let Some(win) = app.get_webview_window(&label) {
        win.set_size(tauri::LogicalSize::new(width, height))
            .map_err(|e| e.to_string())?;
        return Ok(());
    }
    Err("Widget window not found".to_string())
}

#[tauri::command]
fn set_widget_always_on_bottom(
    app: tauri::AppHandle,
    widget_id: String,
    bottom: bool,
) -> Result<(), String> {
    let label = format!("widget_{}", widget_id);
    if let Some(win) = app.get_webview_window(&label) {
        if bottom {
            send_to_desktop_bottom(&win);
            if let Some(main) = app.get_webview_window("main") {
                let _ = main.set_focus();
            }
        } else {
            let _ = win.set_always_on_top(true);
        }
        return Ok(());
    }
    Err("Widget window not found".to_string())
}

#[tauri::command]
fn set_widget_always_on_top(
    app: tauri::AppHandle,
    widget_id: String,
    on_top: bool,
) -> Result<(), String> {
    let label = format!("widget_{}", widget_id);
    if let Some(win) = app.get_webview_window(&label) {
        if on_top {
            let _ = win.set_always_on_top(true);
            let _ = win.set_focus();
        } else {
            send_to_desktop_bottom(&win);
            if let Some(main) = app.get_webview_window("main") {
                let _ = main.set_focus();
            }
        }
        return Ok(());
    }
    Err("Widget window not found".to_string())
}

#[tauri::command]
async fn toggle_all_widget_windows(app: tauri::AppHandle, show: bool) -> Result<(), String> {
    let widget_ids = ["clock", "analogClock", "nextPrayer", "workAlarm", "miniPrayers"];
    for id in widget_ids {
        let label = format!("widget_{}", id);
        if let Some(win) = app.get_webview_window(&label) {
            if show {
                let _ = win.unminimize();
                let _ = win.show();
                send_to_desktop_bottom(&win);
                let _ = win.eval(&format!("try {{ window.__WIDGET_ID__ = '{}'; document.body.style.display = 'flex'; if (typeof initWidget === 'function') initWidget(); }} catch(e){{}}", id));
            } else {
                let _ = win.hide();
            }
        }
    }
    if show {
        if let Some(main) = app.get_webview_window("main") {
            let _ = main.set_focus();
        }
    }
    Ok(())
}

#[repr(C)]
#[derive(Default, Debug, Clone, Copy)]
struct WinRect {
    left: i32,
    top: i32,
    right: i32,
    bottom: i32,
}

#[tauri::command]
fn get_screen_workarea() -> (i32, i32, i32, i32) {
    #[cfg(target_os = "windows")]
    {
        use windows_sys::Win32::UI::WindowsAndMessaging::{SystemParametersInfoW, SPI_GETWORKAREA};
        let mut rc = WinRect::default();
        unsafe {
            let res = SystemParametersInfoW(
                SPI_GETWORKAREA,
                0,
                &mut rc as *mut _ as *mut core::ffi::c_void,
                0,
            );
            if res != 0 {
                return (rc.left, rc.top, rc.right, rc.bottom);
            }
        }
    }
    (0, 0, 1920, 1040)
}

#[tauri::command]
async fn show_popup_notification(
    app: tauri::AppHandle,
    payload: serde_json::Value,
    x: Option<i32>,
    y: Option<i32>,
    width: Option<f64>,
    height: Option<f64>,
) -> Result<(), String> {
    let label = "popup_notification";
    let (default_width, default_height) = match payload.get("type").and_then(|v| v.as_str()) {
        Some("adhan") => (470.0, 310.0),
        Some("alarm") => (450.0, 245.0),
        Some("reminder") => (440.0, 225.0),
        _ => (470.0, 310.0),
    };
    let w = width.unwrap_or(default_width);
    let h = height.unwrap_or(default_height);

    let payload_str = payload.to_string();
    let sanitized_payload = payload_str.replace('\\', "\\\\").replace('\'', "\\'");

    if let Some(win) = app.get_webview_window(label) {
        if let (Some(px), Some(py)) = (x, y) {
            let _ = win.set_position(PhysicalPosition::new(px, py));
        }
        let _ = win.set_size(tauri::LogicalSize::new(w, h));
        let _ = win.unminimize();
        let _ = win.show();
        let _ = win.set_always_on_top(true);
        let _ = win.eval(&format!(
            "try {{ if (window.renderNotification) window.renderNotification(JSON.parse('{}')); }} catch(e){{}}",
            sanitized_payload
        ));
        return Ok(());
    }

    let builder = WebviewWindowBuilder::new(
        &app,
        label,
        WebviewUrl::App("notification.html".into()),
    )
    .title("ميقات - إشعار سطح المكتب")
    .decorations(false)
    .transparent(true)
    .skip_taskbar(true)
    .resizable(false)
    .always_on_top(true)
    .shadow(false)
    .inner_size(w, h);

    let win = builder.build().map_err(|e| e.to_string())?;

    if let (Some(px), Some(py)) = (x, y) {
        let _ = win.set_position(PhysicalPosition::new(px, py));
    }
    let _ = win.unminimize();
    let _ = win.show();
    let _ = win.set_always_on_top(true);
    let _ = win.eval(&format!(
        "try {{ if (window.renderNotification) window.renderNotification(JSON.parse('{}')); }} catch(e){{}}",
        sanitized_payload
    ));

    Ok(())
}

#[tauri::command]
async fn hide_popup_notification(app: tauri::AppHandle) -> Result<(), String> {
    if let Some(win) = app.get_webview_window("popup_notification") {
        let _ = win.hide();
    }
    Ok(())
}

#[tauri::command]
fn quit_app(_app: tauri::AppHandle) {
    std::process::exit(0);
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    std::panic::set_hook(Box::new(|info| {
        use std::io::Write;
        if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open("D:\\Soft\\Miqat\\panic.log") {
            let _ = writeln!(f, "[PANIC] {:?}", info);
        }
    }));

    tauri::Builder::default()
        .plugin(tauri_plugin_log::Builder::default().build())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            // When a second instance is launched, focus the existing main window
            if let Some(win) = app.get_webview_window("main") {
                let _ = win.show();
                let _ = win.unminimize();
                let _ = win.set_focus();
            }
        }))
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            Some(vec!["--minimized"]),
        ))
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .invoke_handler(tauri::generate_handler![
            send_native_notification,
            is_autostart_enabled,
            set_autostart,
            hide_main_window,
            show_main_window,
            minimize_window,
            toggle_maximize_window,
            toggle_main_window,
            toggle_mute,
            get_mute_status,
            open_widget_window,
            close_widget_window,
            toggle_widget_window,
            get_widget_position,
            set_widget_position,
            resize_widget_window,
            set_widget_always_on_bottom,
            set_widget_always_on_top,
            toggle_all_widget_windows,
            drag_widget_window,
            get_screen_workarea,
            show_popup_notification,
            hide_popup_notification,
            quit_app
        ])
        .setup(|app| {
            // Check launch arguments (e.g. --minimized on system boot)
            let args: Vec<String> = std::env::args().collect();
            let is_minimized = args.iter().any(|arg| arg == "--minimized" || arg == "--autostart");

            if let Some(win) = app.get_webview_window("main") {
                // Apply Windows Mica Vibrancy effect if supported
                #[cfg(target_os = "windows")]
                {
                    let _ = window_vibrancy::apply_mica(&win, Some(true));
                }

                if is_minimized {
                    let _ = win.hide();
                } else {
                    let _ = win.show();
                    let _ = win.unminimize();
                    let _ = win.set_focus();
                }
            }

            // Register Global Shortcuts:
            // 1. Ctrl + Shift + M -> Toggle Main Window
            // 2. Ctrl + Shift + Q -> Toggle Mute / Stop Alarm
            // 3. Ctrl + Shift + W -> Toggle Desktop Widgets
            if let Ok(shortcut_m) = "ctrl+shift+m".parse::<Shortcut>() {
                let _ = app.global_shortcut().on_shortcut(shortcut_m, |app, _shortcut, event| {
                    if event.state() == ShortcutState::Pressed {
                        if let Some(win) = app.get_webview_window("main") {
                            if win.is_visible().unwrap_or(false) {
                                let _ = win.hide();
                            } else {
                                let _ = win.show();
                                let _ = win.unminimize();
                                let _ = win.set_focus();
                            }
                        }
                    }
                });
            }

            if let Ok(shortcut_q) = "ctrl+shift+q".parse::<Shortcut>() {
                let _ = app.global_shortcut().on_shortcut(shortcut_q, |app, _shortcut, event| {
                    if event.state() == ShortcutState::Pressed {
                        let current = IS_MUTED.load(Ordering::SeqCst);
                        let new_val = !current;
                        IS_MUTED.store(new_val, Ordering::SeqCst);
                        let _ = app.emit("audio-mute-changed", new_val);
                    }
                });
            }

            if let Ok(shortcut_w) = "ctrl+shift+w".parse::<Shortcut>() {
                let _ = app.global_shortcut().on_shortcut(shortcut_w, |app, _shortcut, event| {
                    if event.state() == ShortcutState::Pressed {
                        let _ = app.emit("tray-toggle-widgets", ());
                    }
                });
            }

            // Build Context Menu for System Tray
            let show_i = MenuItem::with_id(app, "show_main", "إظهار لوحة البرنامج", true, None::<&str>)?;
            let widgets_i = MenuItem::with_id(
                app,
                "toggle_widgets",
                "إظهار / إخفاء ودجات سطح المكتب",
                true,
                None::<&str>,
            )?;
            let mute_i = MenuItem::with_id(app, "toggle_mute", "كتم / تفعيل الصوت", true, None::<&str>)?;
            let quit_i = MenuItem::with_id(app, "quit", "إغلاق البرنامج نهائياً", true, None::<&str>)?;

            let tray_menu = Menu::with_items(app, &[&show_i, &widgets_i, &mute_i, &quit_i])?;

            // Setup Tray Icon
            let icon = app.default_window_icon().cloned().expect("Default window icon missing");
            let _tray = TrayIconBuilder::new()
                .icon(icon)
                .tooltip("ميقات - مواقيت الصلاة ومنبه العمل الذكي")
                .menu(&tray_menu)
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "show_main" => {
                        if let Some(win) = app.get_webview_window("main") {
                            let _ = win.show();
                            let _ = win.unminimize();
                            let _ = win.set_focus();
                        }
                    }
                    "toggle_widgets" => {
                        let _ = app.emit("tray-toggle-widgets", ());
                    }
                    "toggle_mute" => {
                        let current = IS_MUTED.load(Ordering::SeqCst);
                        let new_val = !current;
                        IS_MUTED.store(new_val, Ordering::SeqCst);
                        let _ = app.emit("audio-mute-changed", new_val);
                    }
                    "quit" => {
                        std::process::exit(0);
                    }
                    _ => {}
                })
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    } = event
                    {
                        let app = tray.app_handle();
                        if let Some(win) = app.get_webview_window("main") {
                            if win.is_visible().unwrap_or(false) {
                                let _ = win.hide();
                            } else {
                                let _ = win.show();
                                let _ = win.unminimize();
                                let _ = win.set_focus();
                            }
                        }
                    }
                })
                .build(app)?;

            Ok(())
        })
        .on_window_event(|window, event| {
            // Intercept close button (X) on the main window: hide instead of exiting
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                if window.label() == "main" {
                    api.prevent_close();
                    let _ = window.hide();
                }
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
