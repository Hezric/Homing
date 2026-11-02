#requires -Version 7.0
$ErrorActionPreference = 'Stop'
$projectRoot = Split-Path $PSScriptRoot -Parent
Set-Location -LiteralPath $projectRoot
$version = (yq -p json -o json -r '.version' package.json).Trim()
if ($LASTEXITCODE -or $version -notmatch '^\d+\.\d+\.\d+$') { throw 'Cannot read release version' }
$binary = 'src-tauri/target/release/homing.exe'
if ((Get-Item -LiteralPath $binary).VersionInfo.FileVersion -ne $version) { throw 'Build the matching release executable before packaging' }
$packageRoot = Join-Path $projectRoot "dist/Homing-$version-windows-x64"
$distRoot = [IO.Path]::GetFullPath((Join-Path $projectRoot 'dist')) + [IO.Path]::DirectorySeparatorChar
if (![IO.Path]::GetFullPath($packageRoot).StartsWith($distRoot, [StringComparison]::OrdinalIgnoreCase)) { throw 'Package directory escaped dist' }
# Repack from an empty directory so the previous layout cannot leak into the archive.
if (Test-Path -LiteralPath $packageRoot) { Remove-Item -LiteralPath $packageRoot -Recurse }
$licenseRoot = Join-Path $packageRoot 'licenses'
New-Item -ItemType Directory -Force -Path $licenseRoot | Out-Null
Copy-Item -LiteralPath $binary -Destination $packageRoot
Copy-Item -LiteralPath 'node_modules/linen-ui/packages/linen-ui/glyphs/LICENSE' -Destination (Join-Path $licenseRoot 'Fluent-LICENSE.txt')
Copy-Item -LiteralPath 'node_modules/linen-ui/packages/linen-ui/fonts/Inter-LICENSE.txt','node_modules/linen-ui/packages/linen-ui/fonts/NotoSansSC-LICENSE.txt' -Destination $licenseRoot
$licenseFile = Join-Path $licenseRoot 'THIRD-PARTY-LICENSES.html'
cargo-about generate --manifest-path src-tauri/Cargo.toml --config about.toml --locked --fail --features custom-protocol -o $licenseFile scripts/licenses.hbs
if ($LASTEXITCODE) { throw 'Dependency license generation failed' }
$notices = [IO.File]::ReadAllText($licenseFile)
if ($notices -match '(?im)^\s*Copyright\s*\(c\)\s*&lt;year&gt;') { throw 'Dependency notices contain a generic copyright placeholder' }
$plainNotices = [regex]::Replace([Net.WebUtility]::HtmlDecode([regex]::Replace($notices, '<[^>]+>', ' ')), '\s+', ' ').Trim()
# The generated document must cover these notices before separate duplicate files are omitted.
foreach ($source in @('LICENSE','node_modules/@tauri-apps/api/LICENSE-MIT')) {
    $required = [regex]::Replace([IO.File]::ReadAllText((Join-Path $projectRoot $source)), '\s+', ' ').Trim()
    if (!$plainNotices.Contains($required, [StringComparison]::Ordinal)) { throw "Generated notices do not cover $source" }
}
$encoding = [Text.UTF8Encoding]::new($false)
$noticeIndex = [IO.File]::ReadAllText((Join-Path $projectRoot 'THIRD-PARTY-NOTICES.md')).Replace('(assets/fluent-v1/LICENSE)','(Fluent-LICENSE.txt)').Replace('(public/fonts/Inter-LICENSE.txt)','(Inter-LICENSE.txt)').Replace('(public/fonts/NotoSansSC-LICENSE.txt)','(NotoSansSC-LICENSE.txt)')
[IO.File]::WriteAllText((Join-Path $licenseRoot 'THIRD-PARTY-NOTICES.md'), $noticeIndex, $encoding)
foreach ($readme in @('README.md','README_zh-Hans.md')) {
    $text = [IO.File]::ReadAllText((Join-Path $projectRoot $readme)).Replace('](LICENSE)','](licenses/THIRD-PARTY-LICENSES.html)').Replace('](THIRD-PARTY-NOTICES.md)','](licenses/THIRD-PARTY-NOTICES.md)').Replace('](CONTRIBUTING.md)',"](https://github.com/Hezric/Homing/blob/v$version/CONTRIBUTING.md)").Replace('](docs/features/wifi-reconnect.md)',"](https://github.com/Hezric/Homing/blob/v$version/docs/features/wifi-reconnect.md)")
    $text = $text.Replace('](https://github.com/Hezric/Homing/blob/main/LICENSE)','](licenses/THIRD-PARTY-LICENSES.html)').Replace('](https://github.com/Hezric/Homing/blob/main/THIRD-PARTY-NOTICES.md)','](licenses/THIRD-PARTY-NOTICES.md)')
    [IO.File]::WriteAllText((Join-Path $packageRoot $readme), $text, $encoding)
}
$imageRoot = Join-Path $packageRoot 'docs/images'
New-Item -ItemType Directory -Force -Path $imageRoot | Out-Null
Copy-Item -LiteralPath 'docs/images/flyout.png' -Destination $imageRoot
$archive = Join-Path $projectRoot "dist/Homing-$version-windows-x64.zip"
Compress-Archive -LiteralPath $packageRoot -DestinationPath $archive -Force
Write-Output $archive