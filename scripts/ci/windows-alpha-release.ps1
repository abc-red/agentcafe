$ErrorActionPreference = "Stop"

$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot "..\..")).Path
$versionEnvPath = Join-Path $repoRoot ".drone\version.env"

if (-not (Test-Path $versionEnvPath)) {
    throw "Missing release metadata: $versionEnvPath"
}

$versionLine = Get-Content $versionEnvPath | Where-Object { $_ -match '^(export\s+)?VERSION=' } | Select-Object -First 1
if (-not $versionLine) {
    throw "VERSION is missing from $versionEnvPath"
}

$version = $versionLine -replace '^(export\s+)?VERSION=', ''
$version = $version.Trim('"')

$distDir = Join-Path $repoRoot "dist\alpha\$version"
$publishDir = Join-Path $distDir "windows-publish"
$packageName = "AgentCafe-windows-$version"
$packageDir = Join-Path $distDir $packageName
$zipPath = Join-Path $distDir "$packageName.zip"
$shaPath = Join-Path $distDir "$packageName.zip.sha256"
$artifactDir = Join-Path $repoRoot "artifacts\release\windows"

Set-Location $repoRoot

cargo build -p agentcafe-sidecar --release
dotnet publish apps/windows-wpf/AgentCafe.Windows.csproj -c Release -o $publishDir

if (Test-Path $packageDir) {
    Remove-Item -Recurse -Force $packageDir
}

New-Item -ItemType Directory -Force (Join-Path $packageDir "bin") | Out-Null
Copy-Item -Recurse -Force (Join-Path $publishDir "*") $packageDir
Copy-Item -Force (Join-Path $repoRoot "target\release\agentcafe-sidecar.exe") (Join-Path $packageDir "bin\agentcafe-sidecar.exe")

$readme = @"
Agent Cafe $version Windows controlled Alpha

PowerShell run:
  `$env:AGENTCAFE_SIDECAR="`$PWD\bin\agentcafe-sidecar.exe"
  .\AgentCafe.Windows.exe

This Alpha is read-only. It runs ipc.handshake and doctor.run only.
MVP2 write, snapshot, restore, MCP test, Hook, Plugin, and Skill write actions are disabled.
"@
Set-Content -Path (Join-Path $packageDir "README.txt") -Value $readme -NoNewline

if (Test-Path $zipPath) {
    Remove-Item -Force $zipPath
}

Compress-Archive -Path $packageDir -DestinationPath $zipPath -Force
$hash = (Get-FileHash $zipPath -Algorithm SHA256).Hash
Set-Content -Path $shaPath -Value "$hash  $packageName.zip"

if (Test-Path $artifactDir) {
    Remove-Item -Recurse -Force $artifactDir
}

New-Item -ItemType Directory -Force $artifactDir | Out-Null
Copy-Item -Force $zipPath $artifactDir
Copy-Item -Force $shaPath $artifactDir

Write-Host "Alpha package written to $distDir"
Write-Host "Release artifacts staged at $artifactDir"
