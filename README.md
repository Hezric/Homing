# Homing

A quiet way back to your Wi-Fi.

[简体中文](README_zh-Hans.md)

Homing keeps a saved Windows Wi-Fi network within reach. When its name disappears
from the observed network names, Homing asks Windows to reconnect. WLAN notifications
watch the connection; timers handle retries and the end of a pause.

Its small tray flyout shows the current network, reconnect state, and last
reconnect request. The gear opens settings for the saved profile, a pause of
1–720 minutes, and English or Simplified Chinese. Theme and accent follow Windows.
The badged tray icon distinguishes Homing from Windows Wi-Fi and follows taskbar size.
The centered flyout animates above the taskbar, respects Animation effects off, and
releases its webview after dismissal.

## Build

Windows 11, Rust's MSVC toolchain with Visual Studio C++ Build Tools, Node.js LTS,
and Microsoft Edge WebView2 are required. No release has been published yet.

```powershell
git clone https://github.com/Hezric/Homing.git
cd Homing
npm ci
npm run tauri -- build --ci --no-bundle -- --locked
```

The portable executable is `src-tauri/target/release/homing.exe`. Homing leaves
login startup to your startup manager, such as Lineup.

### Use with an agent

Give this prompt to an agent that can build Windows software:

```text
Build Homing from https://github.com/Hezric/Homing in a local checkout.
Read its AGENTS.md. Check the Windows 11, MSVC Rust/C++ toolchain,
Node.js LTS, and WebView2 prerequisites. Run npm ci, then
npm run tauri -- build --ci --no-bundle -- --locked.
Verify src-tauri/target/release/homing.exe exists without launching it.
Explain that saving a Wi-Fi profile enables reconnect and can switch
networks. Ask before launching with an existing configuration or changing
Wi-Fi on a machine that carries remote sessions. Leave login startup alone.
```

## Use

Click the tray icon to open Homing; right-click for open, pause or resume, and quit.
Open the gear, enter a **saved Wi-Fi profile name**, and save. Windows must already
have that profile and its credentials. Saving enables reconnect immediately.
Homing never creates profiles, scans for networks, or asks for passwords.

Matching preserves the original tool's case-sensitive substring rule in observed
Windows network names. A target `Home` also matches `MyHome5G`. When absent,
Homing runs `netsh wlan connect name=<profile>` without a command shell. An accepted
request is checked after one second; a failed command waits 61 seconds. A successful
request is not proof of a completed connection. **Last reconnect** is the most
recent accepted request in this session. It starts empty after a restart.

Pause presets are 15, 30, 60, and 120 minutes. Custom durations save on Enter or
focus loss. The default is 60 minutes. Editing the duration preserves an active
pause deadline; restarting resumes reconnect. Pause does not disconnect Wi-Fi.

Settings live in `%APPDATA%\Homing\Homing.ini`. The profile is UTF-8 hex-encoded
so Unicode, spaces, and INI punctuation round-trip safely. Changes save atomically.
Missing or malformed settings leave reconnect off; malformed files remain intact
until an explicit settings save. Errors appear in the flyout. If WLAN notifications
are unavailable, Homing uses timed checks while retrying registration, then stops
polling when notifications recover. Wi-Fi adapter status and the original Windows network
description are read without Location permission. Display and matching use that name,
so Windows-added profile-label suffixes do not affect them. If state cannot be read, the tray and flyout show **Can't read Wi-Fi state**.
Homing retries observation without connecting until it knows the target is absent.

## Preview and contribute

```powershell
npm run dev
```

Open `http://127.0.0.1:1420/?lang=en&theme=light&state=connected`.
Browser preview uses mock data and never changes Windows Wi-Fi. Choose `lang=en`
or `lang=zh`, `theme=light` or `theme=dark`, and `state=connected`, `reconnecting`,
`disconnected`, `unreadable`, or `paused`. Use `view=settings` for the settings view.

[Contributing](CONTRIBUTING.md) · [Behavior and checks](docs/features/wifi-reconnect.md)
· [MIT license](LICENSE) · [Third-party notices](THIRD-PARTY-NOTICES.md)
