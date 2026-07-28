<#
.SYNOPSIS
Install the latest delhi release for Windows.

.DESCRIPTION
    irm https://raw.githubusercontent.com/vasanthsarathy/delhi/master/install.ps1 | iex

Downloads one archive from GitHub Releases, verifies it against the published
SHA256SUMS, and unpacks the binary into %LOCALAPPDATA%\delhi\bin. It does not edit
your PATH: it reports whether that directory is already on it and leaves the change
to you.

.PARAMETER Version
Pin a release, e.g. v0.1.0. Defaults to the latest.

.PARAMETER BinDir
Install somewhere other than %LOCALAPPDATA%\delhi\bin.
#>
[CmdletBinding()]
param(
    [string] $Version,
    [string] $BinDir = (Join-Path $env:LOCALAPPDATA 'delhi\bin')
)

$ErrorActionPreference = 'Stop'
$repo = 'vasanthsarathy/delhi'

if (-not $Version) {
    $latest = Invoke-RestMethod "https://api.github.com/repos/$repo/releases/latest"
    $Version = $latest.tag_name
}
if (-not $Version) { throw "could not find the latest release of $repo" }

$number = $Version -replace '^v', ''
$target = 'x86_64-pc-windows-msvc'
$name   = "delhi-$number-$target"
$url    = "https://github.com/$repo/releases/download/$Version/$name.zip"

$tmp = Join-Path ([System.IO.Path]::GetTempPath()) ([System.IO.Path]::GetRandomFileName())
New-Item -ItemType Directory -Path $tmp | Out-Null
try {
    Write-Host "downloading  $name"
    $zip = Join-Path $tmp "$name.zip"
    Invoke-WebRequest -Uri $url -OutFile $zip

    # Verified rather than trusted: a truncated or tampered download should fail loudly
    # instead of installing.
    try {
        $sums = Invoke-WebRequest -Uri "https://github.com/$repo/releases/download/$Version/SHA256SUMS"
        $line = ($sums.Content -split "`n") | Where-Object { $_ -match [regex]::Escape("$name.zip") } | Select-Object -First 1
        if ($line) {
            $want = ($line -split '\s+')[0]
            $have = (Get-FileHash $zip -Algorithm SHA256).Hash.ToLower()
            if ($have -ne $want.ToLower()) {
                throw "checksum mismatch for $name.zip`n  expected $want`n  got      $have"
            }
            Write-Host 'verified     sha256 ok'
        }
    } catch [System.Net.WebException] {
        Write-Warning 'no SHA256SUMS published for this release; skipping verification'
    }

    Expand-Archive -Path $zip -DestinationPath $tmp -Force
    New-Item -ItemType Directory -Path $BinDir -Force | Out-Null
    Copy-Item (Join-Path $tmp "$name\delhi.exe") (Join-Path $BinDir 'delhi.exe') -Force
} finally {
    Remove-Item $tmp -Recurse -Force -ErrorAction SilentlyContinue
}

Write-Host "installed    $(Join-Path $BinDir 'delhi.exe')"
Write-Host ''

$onPath = ($env:PATH -split ';') -contains $BinDir
if ($onPath) {
    Write-Host 'Try:  delhi --help'
} else {
    Write-Host "$BinDir is not on your PATH. To add it for your user account:"
    Write-Host ''
    Write-Host "    [Environment]::SetEnvironmentVariable('PATH', `"`$env:PATH;$BinDir`", 'User')"
    Write-Host ''
    Write-Host "or run it directly:  $(Join-Path $BinDir 'delhi.exe') --help"
}
