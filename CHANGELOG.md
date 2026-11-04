# Changelog

## [0.5.1]

### Fixes

- retain rapid-click flyout openings

## [0.5.0]

### Features

- support portable application data


### Fixes

- bound cargo caches per checkout

## [0.4.4]

### Fixes

- show status-only tooltips (#29)

## [0.4.3]

### Fixes

- smooth settings and retain tray identity

## [0.4.2]

### Fixes

- keep native material active during flyout transitions with Linen 0.1.7

- align icon weight and add presentation checks

## [0.4.1]

### Fixes

- restore radio-off and network-switch recovery (#19)

- consume native geometry and shared surface (#21)

## [0.4.0]

### Features

- adopt toml and stable flyouts (#16)

## [0.3.1]

### Features

- Keep the network input, Save and Use current on one row with equal-size buttons.
- Show a right-side shield and Connected · Protected for a healthy active target connection.

### Fixes

- Clarify that Use current fills a draft and Save commits it in both READMEs and the bundled agent instructions.

## [0.3.0]

### Features

- show protection and align network actions (#12)

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
