# Wi-Fi reconnect

Canonical network and profile terms are defined in [the glossary](../../GLOSSARY.md).

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
- Match the configured network name as a case-sensitive literal substring in the observed
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

`%APPDATA%\Homing\Homing.ini` uses `[Settings]`, `ProfileHex`, `NetworkHex`, `Language`,
and `PauseMinutes`. The internal Windows profile and visible network name use
hex-encoded UTF-8 bytes. Language is system, en,
or zh. Missing or unreadable settings use an empty profile, system language and a
60-minute pause. Malformed files are preserved until an explicit save. Saves sync
a sibling temporary file before renaming; a failed save leaves the active settings
unchanged. Profile changes clear pending retry timing before reconciliation.

## Saved-network selection

The input, suggestions and saved target show the Wi-Fi network name (SSID) used
by Windows, never the internal saved-profile alias. `WlanEnumInterfaces` and
`WlanGetProfileList` enumerate saved entries; `WlanGetProfile` is called with zero
flags to extract only SSID metadata. XML parsing prohibits DTDs and external
entities. Full XML and key material are never logged or retained; a plaintext key
is never requested. Non-UTF-8 SSIDs that cannot be represented reliably are omitted.
Lookup runs on the existing Wi-Fi worker and never scans or connects. The saved-network
command waits asynchronously, leaving native window callbacks responsive.

Every explicit save refreshes the catalog, validates a nonblank network name of
at most 32 UTF-8 bytes, and resolves it to an actual saved profile. If multiple
profiles share a network name, a reliable current entry may disambiguate them;
otherwise saving fails with an explanation instead of choosing a profile.
Malformed/unknown/removed names, enumeration errors and disk-write failures keep
the previous in-memory settings and file. A busy worker leaves an explicit save
pending until its actual result instead of reporting a timeout before a queued
save may commit. `NetworkHex` stores the chosen visible
name and `ProfileHex` retains the internal connection identity. Existing files
without `NetworkHex` remain readable; the worker derives a display name from saved
metadata only when the mapping is unique. This derived name is display-only: legacy
profile substring matching stays unchanged until an explicit target save.
Unavailable legacy names are explained
without showing an internal alias. There is no automatic settings rewrite.

The current-network shortcut first correlates the connected adapter's NLM network
description with that same adapter's saved SSID metadata. A unique mapping works
without Location permission. Only an ambiguous mapping may consult the live WLAN
profile, after `AppCapability.CheckAccess("wiFiControl")` already returns Allowed;
otherwise that query is skipped to avoid a consent prompt. Multiple distinct
current entries or an unavailable mapping disable the shortcut with an explanation.
There is no suffix stripping or guess from another adapter's saved entries.

Matching still uses a case-sensitive literal substring of observed NLM descriptions;
new targets match their visible network name, while connection requests use the
internally resolved profile. The existing retry and pause state machine is unchanged.
Healthy observations do not enumerate saved profiles or query XML. Internal profile
aliases are never added to observed matching text. Legacy targets whose metadata
cannot be resolved retain their established matching behavior; that fallback does
not establish reliable current profile identity.

The ARIA combobox filters case-insensitively without changing the target. Arrows
move the active option; Enter selects it into the draft, and Save commits it.
Enter without an active option saves manual input. Tab closes suggestions without
saving; Escape closes suggestions before a later Escape dismisses the flyout.
IME composition does not select or save. Options expose full names accessibly and
use ellipsis visually. The field label and short current-network button share one
heading row. Suggestions join the input edge-to-edge at its exact width, excluding
the Save column. The input has a single bottom accent only while focused and
returns to neutral on blur. The popup stays above the existing footer and scrolls
internally without increasing panel height. Empty and failed lookups explain their
state, and Retry is keyboard-accessible. The current-network action saves in one
step only after the worker repeats its reliable mapping. A pending save updates the
committed target when it completes but preserves any draft edited after submission.
## Flyout and shell

The current-network card reports the observed Windows network connection separately from the
target profile's reconnect state. A connection to another network remains visibly
connected while the target is retrying.

The shared [Linen shell](https://github.com/Hezric/Linen/blob/v0.1.4/docs/features/tray-shell.md) owns the tray lifecycle, placement, DWM styling and motion; its UI package supplies the shell, status card, pause, language and about controls. The application retains network state, target editing and backend operations. English and Simplified Chinese follow Windows unless explicitly selected. Browser mock mode cannot call Wi-Fi.

An observed connection to the configured target uses Wi-Fi arcs and a loop mark. An absent target, including reconnecting and waiting to retry, uses an exclamation mark; an unconfigured or unreadable target uses the same mark while preserving its status and error text. Pause replaces the mark with two bars and takes precedence over other icon states. The flyout begins directly with the current-network card. Artwork is optically centered and enlarged together for both taskbar sizes; the arcs and three approved marks retain their shapes. No state uses opacity or a no-internet slash. A missing target says “Choose a network to keep”; an absent target names it and explains automatic retries, while unreadable observations retain the existing error text. The current-network card remains distinct from the target's reconnect state. Inter and common assets come from the tagged UI package; Chinese font subsets include both shared and app text.

## Verification

Run `.github/workflows/ci.yml` locally: format, clippy, simulated Rust and frontend
tests, and the production Tauri build. `vp run test:browser` uses Playwright Chromium to
exercise real rendered selection, rejected/stale saves, Unicode, IME, keyboard
scrolling and the footer boundary in both themes and languages. The ignored
`wifi::tests::read_only_saved_networks` diagnostic performs only native read queries
and reports counts/identity availability without printing names. Integration tests cover legacy substring
selection, retry timing and notification coalescing, pause expiry/manual resume,
notification fallback/recovery, settings Unicode round-trip and malformed fallback.
Shell placement tests cover
four taskbar edges, negative monitor coordinates and 100–200% scaling.

Native connection changes require a separate safe hardware session. Never perform
those tests on a network carrying remote sessions. Browser screenshots verify
light/dark, English/Chinese, connected, reconnecting, can’t-reach, unreadable and paused states, and settings without network changes.
