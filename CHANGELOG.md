# Changelog

## [0.2.0]

### Features

- add saved network picker (#9)

## [0.1.1]

### Fixes

- use resolved packaging manifest paths (#4)

- align wifi artwork and native motion (#5)

## [0.1.0]

### Features

- Keep a saved Windows Wi-Fi profile connected with notification-driven observation and the established retry timing.
- Show connection state, the last reconnect request and timed pauses in an English or Simplified Chinese flyout.
- Share the native shell, UI and build tooling through the signed Linen v0.1.1 release.
- Distinguish a watched connection, an absent or unreadable target and a pause with full-contrast tray marks.

### Fixes

- Read Wi-Fi state without Location permission and match original network descriptions without Windows profile-label suffixes.
- Suppress reconnect requests while Wi-Fi state is unreadable or no target is configured.
- Open with the correct theme background, native rounded corners and DWM shadow.
- Toggle and dismiss the flyout consistently, release its webview after closing, and dock hidden-tray flyouts with matching entry and exit motion.
