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
    Win32::System::Com::{CoInitializeEx, CoUninitialize, COINIT_MULTITHREADED},
    UI::ViewManagement::UISettings,
};

#[derive(Clone, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum Action {
    SetNetwork { network: String },
    UseCurrentNetwork,
    SetLanguage { language: String },
    SetPauseMinutes { minutes: u16 },
    TogglePause,
}
pub use linen_tray::Appearance;
use linen_tray::{read_taskbar_small, registry_light};
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
    Networks(Sender<Result<crate::networks::NetworkChoices, String>>),
    Stop,
}
pub struct Worker {
    pub sender: Sender<Message>,
    pub thread: Mutex<Option<JoinHandle<()>>>,
}
impl Worker {
    pub fn request(&self, action: Option<Action>) -> Result<Snapshot, String> {
        let wait_for_save = matches!(
            &action,
            Some(Action::SetNetwork { .. } | Action::UseCurrentNetwork)
        );
        let (send, receive) = mpsc::channel();
        self.sender
            .send(Message::Request(action, Some(send)))
            .map_err(|_| "Wi-Fi watcher is unavailable")?;
        // A queued save may commit after a timeout, so report only its actual result.
        if wait_for_save {
            return receive.recv().map_err(|_| "Wi-Fi watcher is unavailable")?;
        }
        receive
            .recv_timeout(Duration::from_secs(5))
            .map_err(|_| "Wi-Fi watcher did not respond")?
    }
    pub fn networks(&self) -> Result<crate::networks::NetworkChoices, String> {
        let (send, receive) = mpsc::channel();
        self.sender
            .send(Message::Networks(send))
            .map_err(|_| "networksUnavailable")?;
        receive
            .recv_timeout(Duration::from_secs(5))
            .map_err(|_| "networksUnavailable")?
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
    let (mut settings, mut settings_error) = Settings::load(path);
    if settings.network.is_empty() && !settings.profile.is_empty() {
        if let Ok(catalog) = crate::core::WifiBackend::saved_networks(wifi) {
            settings.network = catalog
                .name_for_profile(&settings.profile)
                .unwrap_or_default();
        }
    }
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
            Message::Networks(reply) => {
                let _ = reply.send(
                    crate::core::WifiBackend::saved_networks(wifi).map(|catalog| catalog.choices()),
                );
            }
            Message::NetworkChanged | Message::Retry => reconcile = true,
            Message::AppearanceChanged => appearance = Appearance::read(colors),
            Message::Request(action, response) => {
                reply = response;
                if let Some(action) = action {
                    let mut settings = guard.settings.clone();
                    let persist = !matches!(&action, Action::TogglePause);
                    let target = matches!(
                        &action,
                        Action::SetNetwork { .. } | Action::UseCurrentNetwork
                    );
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
                        Action::SetNetwork { network } => {
                            settings.set_target(Some(&network), wifi, path)
                        }
                        Action::UseCurrentNetwork => settings.set_target(None, wifi, path),
                        Action::TogglePause => {
                            if guard.state(now).pause_remaining > 0 {
                                guard.resume();
                            } else {
                                guard.pause(now);
                            }
                            Ok(())
                        }
                    };
                    match result.and_then(|_| {
                        if persist && !target {
                            settings.save(path)
                        } else {
                            Ok(())
                        }
                    }) {
                        Ok(()) => {
                            if persist {
                                if settings.profile != guard.settings.profile
                                    || settings.network != guard.settings.network
                                {
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
                    appearance.taskbar_small = read_taskbar_small();
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

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn explicit_save_waits_for_the_worker_result_instead_of_reporting_an_uncertain_timeout() {
        let (sender, receive) = mpsc::channel();
        let server = std::thread::spawn(move || {
            if let Message::Request(_, Some(reply)) = receive.recv().unwrap() {
                std::thread::sleep(Duration::from_millis(5100));
                let _ = reply.send(Err("networkNotSaved".into()));
            } else {
                panic!("expected explicit update");
            }
        });
        let worker = Worker {
            sender,
            thread: Mutex::new(None),
        };
        let result = worker.request(Some(Action::SetNetwork {
            network: "Home Wi-Fi".into(),
        }));
        server.join().unwrap();
        assert!(matches!(result, Err(error) if error == "networkNotSaved"));
    }
}
