$ErrorActionPreference = "Stop"
Set-StrictMode -Version Latest

$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot "..\..")).Path
$versionEnvPath = Join-Path $repoRoot ".drone\version.env"

New-Item -ItemType Directory -Force -Path (Split-Path -Parent $versionEnvPath) | Out-Null

if (-not [string]::IsNullOrWhiteSpace($env:DRONE_TAG)) {
    $version = $env:DRONE_TAG
} else {
    $buildId = $env:DRONE_BUILD_NUMBER
    if ([string]::IsNullOrWhiteSpace($buildId)) {
        $buildId = (Get-Date).ToUniversalTime().ToString("yyyyMMddHHmmss")
    }
    $version = "v0.0.0-dev.$buildId"
}

@"
VERSION=$version
MINIO_RELEASE_ROOT=feinian/devops/agentcafe
"@ | Set-Content -LiteralPath $versionEnvPath -Encoding ascii -NoNewline

Write-Host "=== release metadata ==="
Get-Content -LiteralPath $versionEnvPath
