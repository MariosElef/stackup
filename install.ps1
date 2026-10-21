# Install stackup on Windows, in PowerShell:
#   irm https://raw.githubusercontent.com/MariosElef/stackup/main/install.ps1 | iex
# Installs the latest release into %LOCALAPPDATA%\Programs\stackup and adds it to your PATH
# ($env:STACKUP_INSTALL_DIR changes the folder, $env:STACKUP_VERSION picks a version like v0.1.0).
$ErrorActionPreference = "Stop"

$repo = "MariosElef/stackup"
$base = if ($env:STACKUP_RELEASE_BASE) { $env:STACKUP_RELEASE_BASE } else { "https://github.com/$repo/releases" }
$dest = if ($env:STACKUP_INSTALL_DIR) { $env:STACKUP_INSTALL_DIR } else { Join-Path $env:LOCALAPPDATA "Programs\stackup" }

if (-not [Environment]::Is64BitOperatingSystem) { throw "stackup is built for 64-bit Windows" }
$arch = $env:PROCESSOR_ARCHITECTURE
if ($arch -ne "AMD64") { throw "no build for the $arch processor yet" }
$triple = "x86_64-pc-windows-msvc"

$version = $env:STACKUP_VERSION
if (-not $version) {
    # The release page redirects to the newest tag; the tag is the last part of its address.
    $response = Invoke-WebRequest -Uri "$base/latest" -MaximumRedirection 0 -SkipHttpErrorCheck -UseBasicParsing
    $location = [string]$response.Headers.Location
    if (-not $location) { throw "could not find the latest version: set `$env:STACKUP_VERSION, like v0.1.0" }
    $version = $location.Split("/")[-1]
}

$name = "stackup-$version-$triple"
$tmp = Join-Path ([IO.Path]::GetTempPath()) ("stackup-" + [Guid]::NewGuid())
New-Item -ItemType Directory $tmp | Out-Null
try {
    Write-Host "Downloading stackup $version for $triple..."
    Invoke-WebRequest -Uri "$base/download/$version/$name.zip" -OutFile "$tmp\$name.zip" -UseBasicParsing
    Invoke-WebRequest -Uri "$base/download/$version/$name.zip.sha256" -OutFile "$tmp\$name.zip.sha256" -UseBasicParsing

    $expected = ((Get-Content "$tmp\$name.zip.sha256" -Raw).Trim() -split "\s+")[0].ToLower()
    $actual = (Get-FileHash "$tmp\$name.zip" -Algorithm SHA256).Hash.ToLower()
    if ($expected -ne $actual) { throw "the download does not match its checksum: not installing it" }

    Expand-Archive -Path "$tmp\$name.zip" -DestinationPath $tmp
    New-Item -ItemType Directory -Force $dest | Out-Null
    Copy-Item "$tmp\$name\stackup.exe" (Join-Path $dest "stackup.exe") -Force
    Write-Host "Installed $dest\stackup.exe"

    $userPath = [Environment]::GetEnvironmentVariable("Path", "User")
    if (($userPath -split ";") -notcontains $dest) {
        [Environment]::SetEnvironmentVariable("Path", "$userPath;$dest", "User")
        Write-Host "Added $dest to your PATH: open a new terminal to use it."
    }
}
finally {
    Remove-Item -Recurse -Force $tmp -ErrorAction SilentlyContinue
}
