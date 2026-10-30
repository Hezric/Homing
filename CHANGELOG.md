# Changelog

## [Unreleased]

### Changed

- Distinguish every tray state with a Fluent badge and follow Default/Small taskbar sizing.
- Center the flyout on its tray icon and animate appearance, dismissal, content and controls.
- Show and match the original network description without Windows profile-label suffixes.

### Fixed

- Read Wi-Fi state without Location permission using adapter status and Windows network names.
- Suppress reconnects when Wi-Fi state is unreadable and show that state in the tray and flyout.
- Open the flyout asynchronously and defer dismissal until presentation so hidden WebView2 initialization can finish.

## [0.1.0] - 2026-10-30

### Added

- Notification-driven reconnect for a saved Windows Wi-Fi profile.
- A tray flyout with connection state, last reconnect request and timed pause.
- English and Simplified Chinese, system appearance, and browser mock preview.
