---
name: homing
description: Install, inspect or configure Homing on Windows through its native tray UI.
---

# Homing

Download the Windows x64 portable archive from the matching signed release in `Hezric/Homing`. Verify its SHA256 against that release's `SHA256SUMS` before extracting. Keep an existing installation's settings and login registration intact during an update. The archive contains this Skill and its license notices.

Saving a target or starting with an existing target enables automatic Wi-Fi reconnect. Obtain authorization before that activation on a machine carrying remote sessions. First inspect the current connection in the flyout; unreadable is not disconnected. Never test by disconnecting a live network. Browser mock preview is safe.

Settings live in `%APPDATA%\Homing\Homing.ini`; configure the saved Windows profile through the gear and Save. The profile is a case-sensitive substring of the observed network name. “Can’t reach” means the saved target is absent and automatic retries continue. Pause suppresses connect requests without disconnecting. Missing configuration requests a target; unreadable state never enables a connect.

Left-click toggles the flyout; outside clicks and Escape dismiss it. Taskbar icons open a centered flyout; hidden icons open a corner flyout. Right-click offers Open, Pause/Resume and Quit. Installation alone does not authorize a live hardware test.
