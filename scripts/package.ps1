#requires -Version 7.0
$ErrorActionPreference = 'Stop'
$projectRoot = Split-Path $PSScriptRoot -Parent
Set-Location -LiteralPath $projectRoot
$version = (yq -r '.version' package.json).Trim()
if ($LASTEXITCODE) { throw 'Cannot read version' }
$packageRoot = Join-Path $projectRoot "dist/Homing-$version-windows-x64"
New-Item -ItemType Directory -Force -Path $packageRoot | Out-Null
Copy-Item -LiteralPath 'src-tauri/target/release/homing.exe' -Destination $packageRoot
Copy-Item -LiteralPath LICENSE,README.md,README_zh-Hans.md,THIRD-PARTY-NOTICES.md -Destination $packageRoot
Copy-Item -LiteralPath 'assets/fluent-v1/LICENSE' -Destination (Join-Path $packageRoot 'Fluent-LICENSE.txt')
Copy-Item -LiteralPath 'public/fonts/Inter-LICENSE.txt','public/fonts/NotoSansSC-LICENSE.txt' -Destination $packageRoot
cargo-about generate --manifest-path src-tauri/Cargo.toml --config about.toml --locked --fail --features custom-protocol -o (Join-Path $packageRoot 'THIRD-PARTY-LICENSES.html') scripts/licenses.hbs
if ($LASTEXITCODE) { throw 'Dependency license generation failed' }
$notices = [IO.File]::ReadAllText((Join-Path $packageRoot 'THIRD-PARTY-LICENSES.html'))
if ($notices -match '(?im)^\s*Copyright\s*\(c\)\s*&lt;year&gt;') { throw 'Dependency notices contain a generic copyright placeholder; repair the license clarification' }
Copy-Item -LiteralPath 'node_modules/@tauri-apps/api/LICENSE-MIT','node_modules/@tauri-apps/api/LICENSE-APACHE-2.0' -Destination $packageRoot
$archive = Join-Path $projectRoot "dist/Homing-$version-windows-x64.zip"
Compress-Archive -LiteralPath $packageRoot -DestinationPath $archive -Force
Write-Output $archive
