use crate::core::Connection;
use crate::runtime::{Action, Message, Snapshot, Worker};
use tauri::{AppHandle, Manager};

include!("tray_assets.rs");

pub fn install(app: &AppHandle) -> tauri::Result<()> {
    linen_tray::install(
        app,
        linen_tray::Config {
            id: "homing",
            title: "Homing",
            project: "https://github.com/Hezric/Homing",
            artwork: ARTWORK,
            initial: "disconnected",
        },
        |app| {
            if let Some(worker) = app.try_state::<Worker>() {
                let _ = worker
                    .sender
                    .send(Message::Request(Some(Action::TogglePause), None));
            }
        },
    )
}
fn tray_state(state: &crate::core::State) -> &'static str {
    use crate::core::Connection;
    if state.pause_remaining > 0 {
        "paused"
    } else {
        match state.connection {
            Connection::Connected if !state.profile.is_empty() => "connected",
            Connection::Connected | Connection::Reconnecting => "disconnected",
            Connection::Disconnected | Connection::Unreadable => "disconnected",
        }
    }
}

pub fn update_tray(app: &AppHandle, snapshot: &Snapshot) {
    let paused = snapshot.state.pause_remaining > 0;
    let zh = snapshot.appearance.language == "zh";
    let state = tray_state(&snapshot.state);
    let status = if snapshot.state.connection == Connection::Unreadable {
        if zh {
            "无法读取 Wi-Fi 状态"
        } else {
            "Can't read Wi-Fi state"
        }
    } else if paused {
        if zh {
            "重连已暂停"
        } else {
            "Reconnect paused"
        }
    } else if snapshot.state.profile.is_empty() {
        if zh {
            "选择要保持连接的网络"
        } else {
            "Choose a network to keep"
        }
    } else if snapshot.state.connection == Connection::Reconnecting {
        if zh {
            "正在重连…"
        } else {
            "Reconnecting…"
        }
    } else if snapshot.state.connection == Connection::Disconnected {
        ""
    } else if snapshot.state.error.is_some() {
        if zh {
            "重连需要处理"
        } else {
            "Reconnect needs attention"
        }
    } else {
        if zh {
            "自动重连已开启"
        } else {
            "Wi-Fi reconnect is on"
        }
    };

    let retry;
    let status = if status.is_empty() {
        retry = if zh {
            format!(
                "无法连接 {} · 自动重试中",
                snapshot
                    .state
                    .target_name
                    .as_deref()
                    .unwrap_or("已保存的网络")
            )
        } else {
            format!(
                "Can't reach {} · retrying automatically",
                snapshot
                    .state
                    .target_name
                    .as_deref()
                    .unwrap_or("saved network")
            )
        };
        &retry
    } else {
        status
    };
    linen_tray::update_tray(
        app,
        &snapshot.appearance,
        state,
        paused,
        snapshot.state.pause_minutes,
        status,
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{core::Reconnector, settings::Settings};

    #[test]
    fn tray_marks_only_an_observed_target_as_watched() {
        let mut state = Reconnector::new(Settings::default()).state(0);
        for connection in [
            Connection::Connected,
            Connection::Reconnecting,
            Connection::Disconnected,
            Connection::Unreadable,
        ] {
            state.connection = connection;
            state.profile.clear();
            state.pause_remaining = 0;
            assert_eq!(tray_state(&state), "disconnected");
            state.profile = "Home Wi-Fi".into();
            assert_eq!(
                tray_state(&state),
                if connection == Connection::Connected {
                    "connected"
                } else {
                    "disconnected"
                }
            );
            state.pause_remaining = 1;
            assert_eq!(tray_state(&state), "paused");
        }
    }
}
