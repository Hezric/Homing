use crate::runtime::{Action, Message, Snapshot, Worker};
use tauri::{
    image::Image,
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, Manager, WebviewUrl, WebviewWindow, WebviewWindowBuilder,
};

struct TrayItems {
    open: MenuItem<tauri::Wry>,
    pause: MenuItem<tauri::Wry>,
    quit: MenuItem<tauri::Wry>,
}
pub fn install(app: &AppHandle) -> tauri::Result<()> {
    let zh = unsafe { windows::Win32::Globalization::GetUserDefaultUILanguage() & 0x3ff == 4 };
    let open = MenuItem::with_id(
        app,
        "open",
        if zh { "打开 Homing" } else { "Open Homing" },
        true,
        None::<&str>,
    )?;
    let pause = MenuItem::with_id(
        app,
        "pause",
        if zh {
            "暂停 1 小时"
        } else {
            "Pause for 1 hour"
        },
        true,
        None::<&str>,
    )?;
    let quit = MenuItem::with_id(
        app,
        "quit",
        if zh { "退出" } else { "Quit" },
        true,
        None::<&str>,
    )?;
    let menu = Menu::with_items(app, &[&open, &pause, &quit])?;
    app.manage(TrayItems { open, pause, quit });
    TrayIconBuilder::with_id("homing")
        .icon(Image::from_bytes(include_bytes!(
            "../icons/connected-dark/32x32.png"
        ))?)
        .tooltip("Homing")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "open" => {
                open_flyout(app);
            }
            "pause" => {
                if let Some(worker) = app.try_state::<Worker>() {
                    let _ = worker
                        .sender
                        .send(Message::Request(Some(Action::TogglePause), None));
                }
            }
            "quit" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if matches!(
                event,
                TrayIconEvent::Click {
                    button: MouseButton::Left,
                    button_state: MouseButtonState::Up,
                    ..
                }
            ) {
                open_flyout(tray.app_handle());
            }
        })
        .build(app)?;
    Ok(())
}
pub fn open_flyout(app: &AppHandle) {
    let app = app.clone();
    // WebView2 creation deadlocks inside synchronous Windows event handlers.
    tauri::async_runtime::spawn_blocking(move || {
        if let Err(error) = build_flyout(&app) {
            eprintln!("{error}");
        }
    });
}
fn build_flyout(app: &AppHandle) -> tauri::Result<()> {
    if let Some(window) = app.get_webview_window("flyout") {
        window.set_focus()?;
        return Ok(());
    }
    let window = WebviewWindowBuilder::new(app, "flyout", WebviewUrl::App("index.html".into()))
        .title("Homing")
        .inner_size(360.0, 464.0)
        .min_inner_size(300.0, 200.0)
        .background_color(tauri::window::Color(0, 0, 0, 0))
        .decorations(false)
        .transparent(true)
        .shadow(false)
        .resizable(false)
        .skip_taskbar(true)
        .always_on_top(true)
        .visible(false)
        .build()?;
    if let Ok(hwnd) = window.hwnd() {
        use windows::Win32::{Foundation::HWND, Graphics::Dwm::*, UI::WindowsAndMessaging::*};
        unsafe {
            let hwnd = HWND(hwnd.0);
            let style = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);
            SetWindowLongPtrW(
                hwnd,
                GWL_EXSTYLE,
                (style | WS_EX_TOOLWINDOW.0 as isize) & !(WS_EX_APPWINDOW.0 as isize),
            );
            // CSS owns the visible panel; DWM must not outline the full transparent viewport.
            let policy = DWMNCRP_DISABLED;
            let _ = DwmSetWindowAttribute(
                hwnd,
                DWMWA_NCRENDERING_POLICY,
                (&policy as *const DWMNCRENDERINGPOLICY).cast(),
                std::mem::size_of_val(&policy) as u32,
            );
        }
    }
    Ok(())
}
#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FlyoutLayout {
    limit: f64,
    anchor_bottom: f64,
    top_anchored: bool,
}
#[tauri::command]
pub async fn resize_flyout(app: AppHandle, height: f64) -> Result<FlyoutLayout, String> {
    tauri::async_runtime::spawn_blocking(move || resize_window(app, height))
        .await
        .map_err(|e| e.to_string())?
}
fn resize_window(app: AppHandle, height: f64) -> Result<FlyoutLayout, String> {
    if !height.is_finite() {
        return Err("Invalid flyout height".into());
    }
    let window = app
        .get_webview_window("flyout")
        .ok_or("Flyout was closed")?;
    let anchor = app
        .tray_by_id("homing")
        .and_then(|tray| tray.rect().ok().flatten());
    let monitor = match &anchor {
        Some(rect) => {
            let point = rect.position.to_physical::<i32>(1.0);
            app.monitor_from_point(point.x as f64, point.y as f64)
        }
        None => app.primary_monitor(),
    }
    .map_err(|e| e.to_string())?
    .ok_or("No monitor is available")?;
    let scale = monitor.scale_factor();
    let work = monitor.work_area();
    let viewport_height = work.size.height;
    let gap = (12.0 * scale).round() as i32;
    let max_height = work
        .size
        .height
        .saturating_sub(2 * (12.0 * scale).round() as u32) as f64
        / scale;
    let work_rect = [
        work.position.x,
        work.position.y,
        work.position.x + work.size.width as i32,
        work.position.y + work.size.height as i32,
    ];
    let anchor_rect = anchor
        .map(|r| {
            let p = r.position.to_physical::<i32>(scale);
            let s = r.size.to_physical::<u32>(scale);
            [p.x, p.y, p.x + s.width as i32, p.y + s.height as i32]
        })
        .unwrap_or([work_rect[2], work_rect[3], work_rect[2], work_rect[3]]);
    let (x, y, width, panel_height) = placement(work_rect, anchor_rect, scale, height);
    let full = placement(work_rect, anchor_rect, scale, max_height);
    let edge = taskbar_edge(work_rect, anchor_rect);
    let anchor_bottom = if edge == 2 {
        work_rect[3] - gap
    } else {
        anchor_rect[3]
    };
    let layout = FlyoutLayout {
        limit: max_height,
        anchor_bottom: (anchor_bottom - work_rect[1]) as f64 / scale,
        top_anchored: edge == 0,
    };
    // Windows retains an invisible resize frame around the undecorated client area.
    let outer = window.outer_size().map_err(|e| e.to_string())?;
    let inner = window.inner_size().map_err(|e| e.to_string())?;
    let frame_width = outer.width.saturating_sub(inner.width);
    let frame_height = outer.height.saturating_sub(inner.height);
    use windows::Win32::{Foundation::HWND, Graphics::Gdi::*, UI::WindowsAndMessaging::*};
    let hwnd = window.hwnd().map_err(|e| e.to_string())?.0 as isize;
    let (send, receive) = std::sync::mpsc::channel();
    window
        .with_webview(move |_| {
            let result = (|| -> Result<(), String> {
                unsafe {
                    // Keep WebView2's viewport stable; per-frame resizes race its compositor.
                    SetWindowPos(
                        HWND(hwnd as *mut _),
                        None,
                        full.0 - gap - (frame_width / 2) as i32,
                        work_rect[1],
                        width as i32 + 2 * gap + frame_width as i32,
                        viewport_height as i32 + frame_height as i32,
                        SWP_NOACTIVATE | SWP_NOZORDER,
                    )
                    .map_err(|e| e.to_string())?;
                    // Clip drawing and hit testing, leaving room for the panel's CSS shadow.
                    let left = (x - full.0).max(0);
                    let top = (y - work_rect[1] - gap).max(0);
                    let region = CreateRectRgn(
                        left,
                        top,
                        left + width as i32 + 2 * gap + frame_width as i32,
                        (y - work_rect[1] + panel_height as i32 + gap).min(viewport_height as i32),
                    );
                    if region.0.is_null() {
                        return Err("Could not create flyout region".into());
                    }
                    if SetWindowRgn(HWND(hwnd as *mut _), Some(region), true) == 0 {
                        let _ = DeleteObject(region.into());
                        return Err("Could not apply flyout region".into());
                    }
                }
                Ok(())
            })();
            let _ = send.send(result);
        })
        .map_err(|e| e.to_string())?;
    receive
        .recv_timeout(std::time::Duration::from_secs(5))
        .map_err(|_| "Flyout resize did not respond")??;
    Ok(layout)
}
fn tray_state(state: &crate::core::State) -> &'static str {
    use crate::core::Connection;
    if state.pause_remaining > 0 {
        "paused"
    } else {
        match state.connection {
            Connection::Connected => "connected",
            Connection::Reconnecting => "reconnecting",
            Connection::Disconnected | Connection::Unreadable => "disconnected",
        }
    }
}
pub fn update_tray(app: &AppHandle, snapshot: &Snapshot) {
    let paused = snapshot.state.pause_remaining > 0;
    let light = snapshot.appearance.taskbar_light;
    let scale = app
        .tray_by_id("homing")
        .and_then(|t| t.rect().ok().flatten())
        .and_then(|r| {
            let p = r.position.to_physical::<i32>(1.0);
            app.monitor_from_point(p.x as f64, p.y as f64)
                .ok()
                .flatten()
        })
        .map(|m| m.scale_factor())
        .unwrap_or(1.0);
    let size = if scale < 1.125 {
        16
    } else if scale < 1.375 {
        20
    } else if scale < 1.75 {
        24
    } else if scale < 2.5 {
        32
    } else {
        48
    };
    macro_rules! variants {
        ($name:literal) => {{
            macro_rules! sizes {
                ($prefix:literal) => {
                    [
                        include_bytes!(concat!("../icons/", $prefix, $name, "/16x16.png"))
                            .as_slice(),
                        include_bytes!(concat!("../icons/", $prefix, $name, "/20x20.png"))
                            .as_slice(),
                        include_bytes!(concat!("../icons/", $prefix, $name, "/24x24.png"))
                            .as_slice(),
                        include_bytes!(concat!("../icons/", $prefix, $name, "/32x32.png"))
                            .as_slice(),
                        include_bytes!(concat!("../icons/", $prefix, $name, "/48x48.png"))
                            .as_slice(),
                    ]
                };
            }
            if snapshot.appearance.taskbar_small {
                sizes!("small-")
            } else {
                sizes!("")
            }
        }};
    }
    use crate::core::Connection;
    let artwork = match (tray_state(&snapshot.state), light) {
        ("paused", true) => variants!("paused-light"),
        ("paused", false) => variants!("paused-dark"),
        ("connected", true) => variants!("connected-light"),
        ("connected", false) => variants!("connected-dark"),
        ("reconnecting", true) => variants!("reconnecting-light"),
        ("reconnecting", false) => variants!("reconnecting-dark"),
        ("disconnected", true) => variants!("disconnected-light"),
        ("disconnected", false) => variants!("disconnected-dark"),
        _ => unreachable!("tray_state returns a known artwork name"),
    };
    let data = artwork[match size {
        16 => 0,
        20 => 1,
        24 => 2,
        32 => 3,
        _ => 4,
    }];
    let zh = snapshot.appearance.language == "zh";
    let minutes = snapshot.state.pause_minutes;
    let duration = if minutes.is_multiple_of(60) {
        if zh {
            format!("暂停 {} 小时", minutes / 60)
        } else {
            format!("Pause {} hr", minutes / 60)
        }
    } else if zh {
        format!("暂停 {minutes} 分钟")
    } else {
        format!("Pause {minutes} min")
    };
    let label = if paused {
        if zh {
            "立即恢复"
        } else {
            "Resume now"
        }
    } else {
        &duration
    };
    if let Some(items) = app.try_state::<TrayItems>() {
        let _ = items.pause.set_text(label);
        let _ = items
            .open
            .set_text(if zh { "打开 Homing" } else { "Open Homing" });
        let _ = items.quit.set_text(if zh { "退出" } else { "Quit" });
    }
    if let Some(tray) = app.tray_by_id("homing") {
        if let Ok(icon) = Image::from_bytes(data) {
            let _ = tray.set_icon(Some(icon));
        }
        let status = if snapshot.state.connection == Connection::Unreadable {
            if zh {
                "无法读取 Wi-Fi 状态"
            } else {
                "Can't read Wi-Fi state"
            }
        } else if snapshot.state.error.is_some() {
            if zh {
                "重连需要处理"
            } else {
                "Reconnect needs attention"
            }
        } else if paused {
            if zh {
                "重连已暂停"
            } else {
                "Reconnect paused"
            }
        } else {
            if zh {
                "自动重连已开启"
            } else {
                "Wi-Fi reconnect is on"
            }
        };
        let _ = tray.set_tooltip(Some(format!("Homing — {status}")));
    }
}

fn taskbar_edge(work: [i32; 4], anchor: [i32; 4]) -> usize {
    let cx = (anchor[0] + anchor[2]) / 2;
    let cy = (anchor[1] + anchor[3]) / 2;
    [
        (cy - work[1]).abs(),
        (cx - work[2]).abs(),
        (cy - work[3]).abs(),
        (cx - work[0]).abs(),
    ]
    .iter()
    .enumerate()
    .min_by_key(|(_, distance)| *distance)
    .map(|(i, _)| i)
    .unwrap_or(2)
}
fn placement(work: [i32; 4], anchor: [i32; 4], scale: f64, height: f64) -> (i32, i32, u32, u32) {
    let gap = (12.0 * scale).round() as i32;
    let width = (360.0 * scale)
        .round()
        .min((work[2] - work[0] - 2 * gap).max(1) as f64) as i32;
    let height = (height * scale)
        .round()
        .min((work[3] - work[1] - 2 * gap).max(1) as f64)
        .max(1.0) as i32;
    let cx = (anchor[0] + anchor[2]) / 2;
    let edge = taskbar_edge(work, anchor);
    let mut x = cx - width / 2;
    let mut y = anchor[3] - height;
    match edge {
        0 => y = work[1] + gap,
        1 => x = work[2] - width - gap,
        2 => y = work[3] - height - gap,
        _ => x = work[0] + gap,
    }
    (
        x.clamp(work[0] + gap, (work[2] - width - gap).max(work[0] + gap)),
        y.clamp(work[1] + gap, (work[3] - height - gap).max(work[1] + gap)),
        width as u32,
        height as u32,
    )
}
fn animations_enabled() -> bool {
    use windows::{core::BOOL, Win32::UI::WindowsAndMessaging::*};
    let mut enabled = BOOL(0);
    unsafe {
        SystemParametersInfoW(
            SPI_GETCLIENTAREAANIMATION,
            0,
            Some((&mut enabled as *mut BOOL).cast()),
            SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS(0),
        )
        .is_ok()
            && enabled.as_bool()
    }
}
fn animate_flyout(
    window: &WebviewWindow,
    show: bool,
    prepared_style: Option<isize>,
) -> Result<(), String> {
    use std::time::{Duration, Instant};
    use windows::Win32::{
        Foundation::{COLORREF, HWND, RECT},
        UI::WindowsAndMessaging::*,
    };
    let hwnd = HWND(window.hwnd().map_err(|e| e.to_string())?.0);
    unsafe {
        let mut bounds = RECT::default();
        GetWindowRect(hwnd, &mut bounds).map_err(|e| e.to_string())?;
        let current_style = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);
        let style = prepared_style.unwrap_or(current_style);
        if prepared_style.is_some() && style | WS_EX_LAYERED.0 as isize != current_style {
            return Err("Flyout style changed before presentation".into());
        }
        if !animations_enabled() {
            if show {
                SetLayeredWindowAttributes(hwnd, COLORREF(0), 255, LWA_ALPHA)
                    .map_err(|e| e.to_string())?;
                SetWindowLongPtrW(hwnd, GWL_EXSTYLE, style);
                window.show().map_err(|e| e.to_string())?;
            }
            return Ok(());
        }
        SetWindowLongPtrW(hwnd, GWL_EXSTYLE, style | WS_EX_LAYERED.0 as isize);
        let result = (|| {
            SetLayeredWindowAttributes(hwnd, COLORREF(0), if show { 0 } else { 255 }, LWA_ALPHA)?;
            if show {
                let _ = ShowWindow(hwnd, SW_SHOWNOACTIVATE);
            }
            let start = Instant::now();
            loop {
                let t = if animations_enabled() {
                    (start.elapsed().as_secs_f64() / 0.2).min(1.0)
                } else {
                    1.0
                };
                let eased = 1.0 - (1.0 - t).powi(3);
                let visible = if show { eased } else { 1.0 - eased };
                SetLayeredWindowAttributes(
                    hwnd,
                    COLORREF(0),
                    (255.0 * visible).round() as u8,
                    LWA_ALPHA,
                )?;
                SetWindowPos(
                    hwnd,
                    None,
                    bounds.left,
                    bounds.top + (12.0 * (1.0 - visible)).round() as i32,
                    0,
                    0,
                    SWP_NOSIZE | SWP_NOACTIVATE | SWP_NOZORDER,
                )?;
                if t >= 1.0 {
                    break;
                }
                std::thread::sleep(Duration::from_millis(8));
            }
            Ok::<_, windows::core::Error>(())
        })();
        // Restore normal composition before retaining or destroying the webview.
        if !show {
            let _ = ShowWindow(hwnd, SW_HIDE);
        }
        SetWindowLongPtrW(hwnd, GWL_EXSTYLE, style);
        let _ = SetWindowPos(
            hwnd,
            None,
            bounds.left,
            bounds.top,
            0,
            0,
            SWP_NOSIZE | SWP_NOACTIVATE | SWP_NOZORDER,
        );
        result.map_err(|e| e.to_string())
    }
}
#[tauri::command]
pub async fn prepare_flyout(app: AppHandle, height: f64) -> Result<isize, String> {
    tauri::async_runtime::spawn_blocking(move || {
        resize_window(app.clone(), height)?;
        let window = app
            .get_webview_window("flyout")
            .ok_or("Flyout was closed")?;
        use windows::Win32::{
            Foundation::{COLORREF, HWND},
            UI::WindowsAndMessaging::*,
        };
        unsafe {
            let hwnd = HWND(window.hwnd().map_err(|e| e.to_string())?.0);
            let style = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);
            SetWindowLongPtrW(hwnd, GWL_EXSTYLE, style | WS_EX_LAYERED.0 as isize);
            SetLayeredWindowAttributes(hwnd, COLORREF(0), 0, LWA_ALPHA)
                .map_err(|e| e.to_string())?;
            // WebView2 needs visible renderer frames before the first nontransparent native frame.
            let _ = ShowWindow(hwnd, SW_SHOWNOACTIVATE);
            Ok(style)
        }
    })
    .await
    .map_err(|e| e.to_string())?
}
#[tauri::command]
pub async fn present_flyout(app: AppHandle, original_style: isize) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || {
        if let Some(window) = app.get_webview_window("flyout") {
            animate_flyout(&window, true, Some(original_style))?;
            window.set_focus().map_err(|e| e.to_string())?;
            // Initialization can emit focus loss before presentation; dismissal starts afterward.
            let closing = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
            let handle = window.clone();
            window.on_window_event(move |event| {
                if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                    api.prevent_close();
                }
                if matches!(
                    event,
                    tauri::WindowEvent::Focused(false) | tauri::WindowEvent::CloseRequested { .. }
                ) && !closing.swap(true, std::sync::atomic::Ordering::Relaxed)
                {
                    let handle = handle.clone();
                    tauri::async_runtime::spawn_blocking(move || {
                        if let Err(error) = animate_flyout(&handle, false, None) {
                            eprintln!("{error}");
                        }
                        let _ = handle.destroy();
                    });
                }
            });
        }
        Ok(())
    })
    .await
    .map_err(|e| e.to_string())?
}
#[tauri::command]
pub fn dismiss_flyout(app: AppHandle) -> Result<(), String> {
    if let Some(window) = app.get_webview_window("flyout") {
        window.close().map_err(|e| e.to_string())?;
    }
    Ok(())
}
#[tauri::command]
pub fn open_project() -> Result<(), String> {
    use windows::{
        core::w,
        Win32::UI::{Shell::ShellExecuteW, WindowsAndMessaging::SW_SHOWNORMAL},
    };
    let result = unsafe {
        ShellExecuteW(
            None,
            w!("open"),
            w!("https://github.com/Hezric/Homing"),
            None,
            None,
            SW_SHOWNORMAL,
        )
    };
    if result.0 as isize <= 32 {
        Err("Could not open the project page".into())
    } else {
        Ok(())
    }
}
#[cfg(test)]
mod placement_tests {
    use super::{placement, tray_state};
    #[test]
    fn artwork_tracks_connection_and_pause_overrides_every_state() {
        use crate::{
            core::{Connection, Reconnector},
            settings::Settings,
        };
        let mut state = Reconnector::new(Settings::default()).state(0);
        for (connection, expected) in [
            (Connection::Connected, "connected"),
            (Connection::Reconnecting, "reconnecting"),
            (Connection::Disconnected, "disconnected"),
            (Connection::Unreadable, "disconnected"),
        ] {
            state.connection = connection;
            state.pause_remaining = 0;
            assert_eq!(tray_state(&state), expected);
            state.pause_remaining = 1;
            assert_eq!(tray_state(&state), "paused");
        }
    }
    #[test]
    fn centers_and_clamps_at_100_and_150_percent() {
        for (scale, work, anchor, expected) in [
            (
                1.0,
                [0, 0, 1920, 1040],
                [950, 1040, 970, 1080],
                (780, 628, 360, 400),
            ),
            (
                1.5,
                [-2880, 0, 0, 1560],
                [-1455, 1560, -1425, 1620],
                (-1710, 942, 540, 600),
            ),
        ] {
            assert_eq!(placement(work, anchor, scale, 400.0), expected);
            let gap = (12.0_f64 * scale).round() as i32;
            assert_eq!(
                placement(
                    work,
                    [work[0], work[3], work[0] + 20, work[3] + 40],
                    scale,
                    400.0
                )
                .0,
                work[0] + gap
            );
            assert_eq!(
                placement(
                    work,
                    [work[2] - 20, work[3], work[2], work[3] + 40],
                    scale,
                    400.0
                )
                .0,
                work[2] - gap - expected.2 as i32
            );
        }
    }
    #[test]
    fn anchors_at_each_taskbar_edge_with_scaled_gap_on_negative_monitor() {
        for scale in [1.0, 1.25, 1.5, 2.0] {
            let gap = (12.0 * scale) as i32;
            let work = [-2000, 100, -400, 1000];
            for (anchor, edge) in [
                ([-600, 1000, -580, 1020], "bottom"),
                ([-600, 80, -580, 100], "top"),
                ([-2020, 700, -2000, 720], "left"),
                ([-400, 700, -380, 720], "right"),
            ] {
                let (x, y, width, height) = placement(work, anchor, scale, 400.0);
                assert_eq!(width, (360.0 * scale) as u32);
                assert_eq!(height, (400.0 * scale) as u32);
                assert!(
                    x >= work[0] + gap
                        && y >= work[1] + gap
                        && x + width as i32 <= work[2] - gap
                        && y + height as i32 <= work[3] - gap
                );
                match edge {
                    "bottom" => assert_eq!(y + height as i32, work[3] - gap),
                    "top" => assert_eq!(y, work[1] + gap),
                    "left" => assert_eq!(x, work[0] + gap),
                    _ => assert_eq!(x + width as i32, work[2] - gap),
                }
            }
        }
    }
}
