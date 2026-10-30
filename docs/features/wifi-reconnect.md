# Wi-Fi reconnect

Homing monitors a single saved Windows Wi-Fi profile. `WifiBackend` is the boundary
between the deterministic reconnect core and Windows. Tests supply a simulated
backend; they never issue native network commands.

## Reconnect rules

The compatibility reference is the predecessor's configurable AHK tool. Its CMD
companion has the same selection rule but a two-second loop; the AHK's one-second
loop and extra sixty-second wait after command failure govern Homing.

- Read Wi-Fi adapter operational status with IP Helper and correlate connected
  Network List Manager connections by adapter GUID. These APIs work with Location
  services off. Use the network description (`INetwork::GetDescription`) for both
  display and matching, rather than the editable connection-profile label
  (`GetName`), to avoid Windows-added numeric suffixes. Preserve actual numeric
  endings; an unavailable description is unreadable and never enables reconnect.
- Match the configured name as a case-sensitive literal substring in the observed
  network names. A target `Home` also matches `MyHome5G`.
- Reconnect only when all Wi-Fi adapters can be observed and the target is absent:
  adapters are disconnected or connected to another named network. Unknown status,
  missing adapters, a connected adapter without a readable name, and API failures
  enter **Can't read Wi-Fi state**, clear stale network text, and never connect.
  Observation retries after one second, including while paused or unconfigured.
- For a known absent target, request `netsh wlan connect name=<profile>`. Use
  separate process arguments and an absolute system executable path.
- An accepted request enters reconnecting and schedules verification after one
  second. Command failure enters disconnected and schedules retry after 61 seconds.
- Notifications refresh observed state but never shorten a pending retry cooldown.
  Observing the target cancels retry timing immediately. A healthy connection has
  no polling timer when notifications are available. Last reconnect records accepted
  requests, not confirmed success, in Unix seconds for this process lifetime.
- A missing profile disables reconnect. Profile validation rejects blank names,
  embedded NUL/newlines, and names longer than 255 UTF-16 code units.

The worker subscribes to Windows WLAN ACM notifications. Callback context stays
allocated until notification unregistration and handle closure complete; callbacks
only send a message. Adapter/profile observations and netsh connect requests execute on the worker.

Failed notification registration keeps the app running with a visible error and
retries registration during inspection. While notifications are unavailable,
a matching target uses the legacy one-second observation loop; absent targets
retain the normal one-second/61-second request timing. Paused fallback checks only
observe, and pause expiry still resumes on time. An unconfigured app retries
registration after 61 seconds without issuing connect requests. Successful
registration removes healthy-state polling immediately.

## Pause and settings

A pause suppresses every connect request until its monotonic deadline. Notifications
continue updating the observed network. Manual resume reconciles immediately, and
expiry also reconciles without needing a notification. The default is 60 minutes;
valid saved values are whole minutes from 1 to 720. Editing the saved duration
preserves the active deadline. Pauses and reconnect timestamps are session state.

`%APPDATA%\Homing\Homing.ini` uses `[Settings]`, `ProfileHex`, `Language`, and
`PauseMinutes`. ProfileHex holds hex-encoded UTF-8 bytes; Language is system, en,
or zh. Missing or unreadable settings use an empty profile, system language and a
60-minute pause. Malformed files are preserved until an explicit save. Saves sync
a sibling temporary file before renaming; a failed save leaves the active settings
unchanged. Profile changes clear pending retry timing before reconciliation.

## Flyout and shell

The current-network card reports the observed Windows network connection separately from the
target profile's reconnect state. A connection to another network remains visibly
connected while the target is retrying.

The Earmuff frontend and shell were copied and adapted. The flyout uses its local
Inter/Noto Sans SC fonts, theme/accent tokens, header, current card, list, footer,
settings swap, pause presets and custom field. English and Simplified Chinese
follow Windows display language unless explicitly selected. Theme and taskbar
artwork refresh on system color notifications.

The tray has connected, reconnecting, disconnected, and paused artwork in light and
dark variants. Every state carries a Fluent knockout badge: sync for connected,
reconnecting and disconnected, pause while paused. Reconnecting dims only the
Wi-Fi glyph to 40%. Each ICO holds 16/20/24/32/48 px entries, selected according
to monitor scaling. Small-taskbar artwork uses 75% of the same bitmap canvas.
The worker refreshes taskbar size/theme and monitor DPI every second without
querying Wi-Fi; `TaskbarSize` governs newer Windows builds, with `TaskbarSi`
as the legacy fallback. `IconSizePreference` affects app buttons, not tray glyphs. Left-click
opens the flyout; right-click offers open, pause/resume and quit. Opening a second
instance opens the existing instance's flyout.

The undecorated tool window centers on the tray icon, follows its monitor, taskbar
edge and DPI, and clamps inside the work area. Its transparent viewport stays
stable while content height animates, with a native region bounding drawing and
hit testing. The panel stays anchored above the taskbar. Presentation primes
WebView2 at zero alpha after fonts are ready, then fades/slides in over 200 ms;
focus loss or Esc animates out before destroying the webview. Settings, messages,
header/text, pause, toggles and hover/press changes use short ease-out transitions.
Windows Animation effects off and reduced-motion preferences make changes instant.
Webview creation runs outside synchronous Windows tray event handlers to avoid
blocking the shell. Focus-loss dismissal begins after first presentation so hidden
WebView2 initialization can complete. Startup registration is external. Browser mock mode cannot call Windows Wi-Fi;
its preview controls are documented in the README.

## Verification

Run `.github/workflows/ci.yml` locally: format, clippy, simulated Rust and frontend
tests, and the production Tauri build. Integration tests cover legacy substring
selection, retry timing and notification coalescing, pause expiry/manual resume,
notification fallback/recovery, settings Unicode round-trip and malformed fallback.
Shell placement tests cover
four taskbar edges, negative monitor coordinates and 100–200% scaling.

Native connection changes require a separate safe hardware session. Never perform
those tests on a network carrying remote sessions. Browser screenshots verify
light/dark, English/Chinese, connected, reconnecting, disconnected, unreadable and paused states, and settings without network changes.
