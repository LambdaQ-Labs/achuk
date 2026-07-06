# Achuk installer for Windows — https://achuk.dev
#
#   irm https://achuk.dev/install.ps1 | iex
#
# Downloads the latest Achuk release, unpacks it into %USERPROFILE%\.achuk,
# and puts `achuk` on your PATH. No admin rights needed.

$ErrorActionPreference = "Stop"
$Repo   = "LambdaQ-Labs/achuk"
$Prefix = if ($env:ACHUK_HOME) { $env:ACHUK_HOME } else { Join-Path $env:USERPROFILE ".achuk" }
$BinDir = Join-Path $Prefix "bin"

function Say($m)  { Write-Host "achuk " -ForegroundColor Cyan -NoNewline; Write-Host $m }
function Die($m)  { Write-Host "error " -ForegroundColor Red -NoNewline; Write-Host $m; exit 1 }

# --- platform check --------------------------------------------------------
if (-not [Environment]::Is64BitOperatingSystem) {
  Die "Achuk needs 64-bit Windows (x64)."
}

# --- resolve version -------------------------------------------------------
$Version = if ($env:ACHUK_VERSION) { $env:ACHUK_VERSION } else { "latest" }
if ($Version -eq "latest") {
  try {
    $rel = Invoke-RestMethod "https://api.github.com/repos/$Repo/releases/latest" `
      -Headers @{ "User-Agent" = "achuk-install" }
    $Version = $rel.tag_name
  } catch { Die "could not determine the latest release (set `$env:ACHUK_VERSION)" }
}
if (-not $Version) { Die "could not resolve a version" }

$Asset = "achuk-$Version-windows-x64.zip"
$Url   = "https://github.com/$Repo/releases/download/$Version/$Asset"

# --- download + unpack -----------------------------------------------------
Say "installing $Version for windows-x64"
$tmp = Join-Path $env:TEMP ("achuk-" + [System.Guid]::NewGuid().ToString("N"))
New-Item -ItemType Directory -Path $tmp -Force | Out-Null
$zip = Join-Path $tmp $Asset
try {
  Invoke-WebRequest -Uri $Url -OutFile $zip -Headers @{ "User-Agent" = "achuk-install" }
} catch { Die "download failed: $Url" }

if (Test-Path $Prefix) { Remove-Item -Recurse -Force $Prefix }
New-Item -ItemType Directory -Path $Prefix -Force | Out-Null
try {
  Expand-Archive -Path $zip -DestinationPath $Prefix -Force
} catch { Die "unpack failed" }
Remove-Item -Recurse -Force $tmp

$exe = Join-Path $BinDir "achuk.exe"
if (-not (Test-Path $exe)) { Die "installed binary not found at $exe" }

# --- PATH (user scope, no admin) -------------------------------------------
$userPath = [Environment]::GetEnvironmentVariable("Path", "User")
if ($userPath -notlike "*$BinDir*") {
  $newPath = if ($userPath) { "$userPath;$BinDir" } else { $BinDir }
  [Environment]::SetEnvironmentVariable("Path", $newPath, "User")
  $env:Path = "$env:Path;$BinDir"
  Say "added $BinDir to your PATH"
}

$ver = & $exe --version
Say "installed $ver"
Write-Host ""
Write-Host "  Get started:"
Write-Host "    achuk new hello"
Write-Host "    cd hello"
Write-Host "    achuk run"
Write-Host ""
Write-Host "  (Open a NEW terminal so the updated PATH takes effect.)"
