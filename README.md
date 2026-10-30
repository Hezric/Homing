# Homing

A quiet way back to your Wi-Fi.

[简体中文](README_zh-Hans.md)

Homing keeps a saved Windows Wi-Fi network within reach. When its name disappears
from the observed network names, Homing asks Windows to reconnect. WLAN notifications
watch the connection; timers handle retries and the end of a pause.

Its small tray flyout shows the current network, reconnect state, and last
reconnect request. The gear opens settings for the saved profile, a pause of
1–720 minutes, and English or Simplified Chinese. Theme and accent follow Windows.
The loop-marked tray icon distinguishes Homing from Windows Wi-Fi and follows taskbar size.
The flyout centers above a taskbar icon, or slides into the notification corner
from a hidden icon. Windows supplies its shadow and corners. Outside clicks, Escape
and another tray click dismiss it; Animation effects off makes changes instant.

## Install

Download the Windows x64 portable archive from [Releases](https://github.com/Hezric/Homing/releases/latest),
verify it against `SHA256SUMS`, and extract it. Keep WebView2 installed. Saving a target enables automatic reconnect.
Login startup remains with your startup manager.

## Build

Windows 11, Rust's MSVC toolchain with Visual Studio C++ Build Tools, Node.js LTS,
Microsoft Edge WebView2 and [Vite+](https://viteplus.dev/guide/) are required.

```powershell
git clone https://github.com/Hezric/Homing.git
cd Homing
vp install --frozen-lockfile
vp exec tauri build --ci --no-bundle -- --locked
```

The portable executable is `src-tauri/target/release/homing.exe`. Homing leaves
login startup to your startup manager, such as Lineup.

### Use with an agent

Give this prompt to an agent that can build Windows software:

```text
Install Homing from https://github.com/Hezric/Homing/releases/latest.
Verify the Windows x64 archive against its SHA256SUMS, extract it, and check
that homing.exe exists without launching it. Preserve existing settings
and startup registration. Explain that saving a target or using an existing target enables Wi-Fi reconnect; obtain authorization before activation.
Install skills/homing through my existing Skill management. If I have
none, ask first and recommend `npx skills add Hezric/Homing -g` using Vercel's
skills CLI (https://github.com/vercel-labs/skills; Node.js 22.20+).
Verify the installed Skill's path and complete contents too.
```

## Use

Click the tray icon to toggle Homing; right-click for open, pause or resume, and quit.
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

**Can't reach** means the saved target is absent and retries continue automatically.
An empty target asks you to choose a network; **Reconnecting** means a request is
awaiting confirmation. Solid artwork means working, outline means waiting, and
pause uses two bars. All tray states retain full contrast.

## Preview and contribute

```powershell
vp run dev
```

Open `http://127.0.0.1:1420/?lang=en&theme=light&state=connected`.
Browser preview uses mock data and never changes Windows Wi-Fi. Choose `lang=en`
or `lang=zh`, `theme=light` or `theme=dark`, and `state=connected`, `reconnecting`,
`disconnected`, `unreadable`, or `paused`. Use `view=settings` for the settings view.

[Contributing](CONTRIBUTING.md) · [Behavior and checks](docs/features/wifi-reconnect.md)
· [MIT license](LICENSE) · [Third-party notices](THIRD-PARTY-NOTICES.md)
