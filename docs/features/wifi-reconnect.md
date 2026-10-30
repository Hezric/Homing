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
  second. Command failure enters the can’t-reach state and schedules retry after 61 seconds.
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

The shared [Linen shell](https://github.com/Hezric/Linen/blob/v0.1.0/docs/features/tray-shell.md) owns the tray lifecycle, placement, DWM styling and motion; its UI package supplies the shell, status card, pause, language and about controls. The application retains network state, target editing and backend operations. English and Simplified Chinese follow Windows unless explicitly selected. Browser mock mode cannot call Wi-Fi.

Connected uses solid Wi-Fi arcs and a loop mark. “Can't reach” uses full-contrast outline arcs with the same loop; reconnecting fills the arcs over a repeating short sequence. Pause replaces the loop with two bars. No state uses opacity or a no-internet slash. A missing target says “Choose a network to keep”; an absent target names it and explains automatic retries, while unreadable observations retain the existing error text. The current-network card remains distinct from the target's reconnect state. Inter and common assets come from the tagged UI package; Chinese font subsets include both shared and app text.

## Verification

Run `.github/workflows/ci.yml` locally: format, clippy, simulated Rust and frontend
tests, and the production Tauri build. Integration tests cover legacy substring
selection, retry timing and notification coalescing, pause expiry/manual resume,
notification fallback/recovery, settings Unicode round-trip and malformed fallback.
Shell placement tests cover
four taskbar edges, negative monitor coordinates and 100–200% scaling.

Native connection changes require a separate safe hardware session. Never perform
those tests on a network carrying remote sessions. Browser screenshots verify
light/dark, English/Chinese, connected, reconnecting, can’t-reach, unreadable and paused states, and settings without network changes.
