use crate::{
    core::{Reconnector, State},
    settings::Settings,
    shell,
    wifi::WindowsWifi,
};
use serde::{Deserialize, Serialize};
use std::{
    path::PathBuf,
    sync::{
        mpsc::{self, Receiver, Sender},
        Mutex,
    },
    thread::JoinHandle,
    time::{Duration, Instant},
};
use tauri::{AppHandle, Emitter};
use windows::{
    core::w,
    Foundation::TypedEventHandler,
    Win32::{
        Globalization::GetUserDefaultUILanguage,
        System::{
            Com::{CoInitializeEx, CoUninitialize, COINIT_MULTITHREADED},
            Registry::*,
        },
    },
    UI::ViewManagement::{UIColorType, UISettings},
};

#[derive(Clone, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum Action {
    SetProfile { profile: String },
    SetLanguage { language: String },
    SetPauseMinutes { minutes: u16 },
    TogglePause,
}
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Appearance {
    pub language: String,
    pub dark: bool,
    pub accent: String,
    pub accent_ink: String,
    pub taskbar_light: bool,
    pub taskbar_small: bool,
}
impl Appearance {
    pub fn read(colors: &UISettings) -> Self {
        let color = colors.GetColorValue(UIColorType::Accent).ok();
        let accent = color
            .map(|c| format!("#{:02x}{:02x}{:02x}", c.R, c.G, c.B))
            .unwrap_or("#0067c0".into());
        let accent_ink = color
            .map(|c| {
                if 299 * c.R as u32 + 587 * c.G as u32 + 114 * c.B as u32 > 150000 {
                    "#000000"
                } else {
                    "#ffffff"
                }
            })
            .unwrap_or("#ffffff")
            .into();
        let language = unsafe {
            if GetUserDefaultUILanguage() & 0x3ff == 4 {
                "zh"
            } else {
                "en"
            }
        }
        .into();
        Self {
            language,
            dark: registry_light(w!("AppsUseLightTheme")) == 0,
            taskbar_light: registry_light(w!("SystemUsesLightTheme")) != 0,
            taskbar_small: taskbar_small(),
            accent,
            accent_ink,
        }
    }
}
fn registry_light(name: windows::core::PCWSTR) -> u32 {
    let mut value = 1u32;
    unsafe {
        let _ = RegGetValueW(
            HKEY_CURRENT_USER,
            w!("Software\\Microsoft\\Windows\\CurrentVersion\\Themes\\Personalize"),
            name,
            RRF_RT_REG_DWORD,
            None,
            Some((&mut value as *mut u32).cast()),
            Some(&mut 4),
        );
    }
    value
}
fn taskbar_small() -> bool {
    let read = |name| {
        let mut value = 0u32;
        unsafe {
            RegGetValueW(
                HKEY_CURRENT_USER,
                w!("Software\\Microsoft\\Windows\\CurrentVersion\\Explorer\\Advanced"),
                name,
                RRF_RT_REG_DWORD,
                None,
                Some((&mut value as *mut u32).cast()),
                Some(&mut 4),
            )
            .is_ok()
            .then_some(value)
        }
    };
    // shortcut: TaskbarSize is undocumented; revisit if Windows changes the tray modes.
    // IconSizePreference sizes app buttons, not notification-area glyphs.
    match read(w!("TaskbarSize")) {
        Some(value) => value == 1,
        None => read(w!("TaskbarSi")) == Some(0),
    }
}
#[derive(Clone, Serialize)]
pub struct Snapshot {
    #[serde(flatten)]
    pub state: State,
    #[serde(flatten)]
    pub appearance: Appearance,
}
pub enum Message {
    NetworkChanged,
    Retry,
    AppearanceChanged,
    Request(Option<Action>, Option<Sender<Result<Snapshot, String>>>),
    Stop,
}
pub struct Worker {
    pub sender: Sender<Message>,
    pub thread: Mutex<Option<JoinHandle<()>>>,
}
impl Worker {
    pub fn request(&self, action: Option<Action>) -> Result<Snapshot, String> {
        let (send, receive) = mpsc::channel();
        self.sender
            .send(Message::Request(action, Some(send)))
            .map_err(|_| "Wi-Fi watcher is unavailable")?;
        receive
            .recv_timeout(Duration::from_secs(5))
            .map_err(|_| "Wi-Fi watcher did not respond")?
    }
    pub fn stop(&self) {
        let _ = self.sender.send(Message::Stop);
        if let Some(thread) = self.thread.lock().ok().and_then(|mut t| t.take()) {
            let _ = thread.join();
        }
    }
}
struct Apartment;
impl Drop for Apartment {
    fn drop(&mut self) {
        unsafe {
            CoUninitialize();
        }
    }
}
pub fn start(app: AppHandle, path: PathBuf) -> Result<Worker, String> {
    let (sender, receive) = mpsc::channel();
    let thread_sender = sender.clone();
    let (ready_send, ready_receive) = mpsc::channel();
    let thread = std::thread::Builder::new()
        .name("homing-wifi".into())
        .spawn(move || {
            let run = || -> Result<(), String> {
                unsafe {
                    CoInitializeEx(None, COINIT_MULTITHREADED)
                        .ok()
                        .map_err(|e| e.to_string())?;
                }
                let _apartment = Apartment;
                let mut wifi = WindowsWifi::new(thread_sender.clone());
                let colors = UISettings::new().map_err(|e| e.to_string())?;
                let appearance_sender = thread_sender.clone();
                let cookie = colors
                    .ColorValuesChanged(&TypedEventHandler::new(move |_, _| {
                        let _ = appearance_sender.send(Message::AppearanceChanged);
                        Ok(())
                    }))
                    .map_err(|e| e.to_string())?;
                let _ = ready_send.send(Ok(()));
                run_loop(&app, &path, receive, &mut wifi, &colors);
                let _ = colors.RemoveColorValuesChanged(cookie);
                Ok(())
            };
            if let Err(error) = run() {
                let _ = ready_send.send(Err(error));
            }
        })
        .map_err(|e| e.to_string())?;
    match ready_receive.recv().map_err(|e| e.to_string())? {
        Ok(()) => Ok(Worker {
            sender,
            thread: Mutex::new(Some(thread)),
        }),
        Err(error) => {
            let _ = thread.join();
            Err(error)
        }
    }
}
fn run_loop(
    app: &AppHandle,
    path: &std::path::Path,
    receive: Receiver<Message>,
    wifi: &mut WindowsWifi,
    colors: &UISettings,
) {
    let (settings, mut settings_error) = Settings::load(path);
    let mut guard = Reconnector::new(settings);
    let clock = Instant::now();
    let mut appearance = Appearance::read(colors);
    let mut next = Message::NetworkChanged;
    loop {
        let now = clock.elapsed().as_millis() as u64;
        let mut reply = None;
        let mut action_error = None;
        let mut reconcile = false;
        match next {
            Message::Stop => break,
            Message::NetworkChanged | Message::Retry => reconcile = true,
            Message::AppearanceChanged => appearance = Appearance::read(colors),
            Message::Request(action, response) => {
                reply = response;
                if let Some(action) = action {
                    let mut settings = guard.settings.clone();
                    let persist = !matches!(&action, Action::TogglePause);
                    let result = match action {
                        Action::SetLanguage { language } => {
                            if ["system", "zh", "en"].contains(&language.as_str()) {
                                settings.language = language;
                                Ok(())
                            } else {
                                Err("Invalid language".into())
                            }
                        }
                        Action::SetPauseMinutes { minutes } => {
                            if (1..=720).contains(&minutes) {
                                settings.pause_minutes = minutes;
                                Ok(())
                            } else {
                                Err("Pause duration must be 1 to 720 whole minutes".into())
                            }
                        }
                        Action::SetProfile { profile } => {
                            crate::settings::validate_profile(&profile)
                                .map(|_| settings.profile = profile)
                        }
                        Action::TogglePause => {
                            if guard.state(now).pause_remaining > 0 {
                                guard.resume();
                            } else {
                                guard.pause(now);
                            }
                            Ok(())
                        }
                    };
                    match result.and_then(|_| if persist { settings.save(path) } else { Ok(()) }) {
                        Ok(()) => {
                            if persist {
                                if settings.profile != guard.settings.profile {
                                    guard.reset_retry();
                                }
                                guard.settings = settings;
                                settings_error = None;
                            }
                            reconcile = true;
                        }
                        Err(e) => action_error = Some(e),
                    }
                }
            }
        }
        if reconcile {
            let unix = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();
            guard.reconcile(wifi, || clock.elapsed().as_millis() as u64, unix);
        }
        let mut state = guard.state(now);
        state.error = state.error.or(settings_error.clone());
        let mut resolved = appearance.clone();
        if guard.settings.language != "system" {
            resolved.language = guard.settings.language.clone();
        }
        let snapshot = Snapshot {
            state,
            appearance: resolved,
        };
        let tray_app = app.clone();
        let tray_snapshot = snapshot.clone();
        let _ = app.run_on_main_thread(move || shell::update_tray(&tray_app, &tray_snapshot));
        let _ = app.emit("state-changed", &snapshot);
        if let Some(reply) = reply {
            let _ = reply.send(action_error.map_or_else(|| Ok(snapshot.clone()), Err));
        }
        next = loop {
            let now = clock.elapsed().as_millis() as u64;
            let wait = guard
                .deadline(now)
                .map(|deadline| deadline.saturating_sub(now))
                .unwrap_or(1000)
                .min(1000);
            match receive.recv_timeout(Duration::from_millis(wait)) {
                Ok(message) => break message,
                Err(mpsc::RecvTimeoutError::Disconnected) => return,
                Err(mpsc::RecvTimeoutError::Timeout) => {
                    if guard
                        .deadline(clock.elapsed().as_millis() as u64)
                        .is_some_and(|deadline| deadline <= clock.elapsed().as_millis() as u64)
                    {
                        break Message::Retry;
                    }
                    // Refresh shell artwork for live taskbar/DPI changes without inspecting Wi-Fi.
                    appearance.taskbar_small = taskbar_small();
                    appearance.taskbar_light = registry_light(w!("SystemUsesLightTheme")) != 0;
                    let mut idle = snapshot.clone();
                    idle.appearance.taskbar_small = appearance.taskbar_small;
                    idle.appearance.taskbar_light = appearance.taskbar_light;
                    let app = app.clone();
                    let _ = app
                        .clone()
                        .run_on_main_thread(move || shell::update_tray(&app, &idle));
                }
            }
        };
    }
}
