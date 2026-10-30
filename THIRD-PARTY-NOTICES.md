# Third-party notices

The bundled artwork in `assets/fluent-v1/` is from Microsoft Fluent UI System
Icons, `@fluentui/svg-icons` 1.1.340, under its [MIT license](assets/fluent-v1/LICENSE).
Tray artwork uses the 20/24/48 px Wi-Fi or Wi-Fi-off source variants, composed
separately at each output size with Fluent pause or sync badges. The colored app
icon adapts the Wi-Fi glyph. Frontend and shell styling were adapted from Earmuff,
Copyright (c) 2026 Hezric, under the repository's MIT license.

Inter and the Noto Sans SC UI subset are bundled under the SIL Open Font License:
[Inter](public/fonts/Inter-LICENSE.txt) and [Noto Sans SC](public/fonts/NotoSansSC-LICENSE.txt).
`scripts/subset-fonts.py` reproduces the Chinese subset from the locked Fontsource
package and UI text in `src/state.js`.

Release archives include `THIRD-PARTY-LICENSES.html`, generated with cargo-about
from the Windows runtime dependency graph, alongside the bundled fonts' and
icons' licenses and the Tauri frontend API's MIT and Apache licenses.
Exact dependency versions are recorded in the Cargo and npm lockfiles.
