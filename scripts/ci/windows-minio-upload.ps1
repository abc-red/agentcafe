[CmdletBinding()]
param(
    [ValidateSet("verify-minio", "upload")]
    [string]$Action = "upload",
    [string]$LocalDir,
    [string]$ArtifactKind = "windows"
)

$ErrorActionPreference = "Stop"
Set-StrictMode -Version Latest

$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot "..\..")).Path
$versionEnvPath = Join-Path $repoRoot ".drone\version.env"
$mcConfigDir = if ([string]::IsNullOrWhiteSpace($env:AGENTCAFE_MC_CONFIG_DIR)) {
    "C:\build-tools\mconfig"
} else {
    $env:AGENTCAFE_MC_CONFIG_DIR
}
$mcConfigFile = Join-Path $mcConfigDir "config.json"
$mcCommand = if ([string]::IsNullOrWhiteSpace($env:AGENTCAFE_MC_COMMAND)) {
    "mc"
} else {
    $env:AGENTCAFE_MC_COMMAND
}

function Invoke-Checked {
    param(
        [Parameter(Mandatory = $true)][string]$Command,
        [Parameter()][object[]]$CommandArgs = @()
    )

    & $Command @CommandArgs
    $exitCode = if (Test-Path -LiteralPath "Variable:\LASTEXITCODE") { $LASTEXITCODE } else { 0 }
    if (-not $? -or $exitCode -ne 0) {
        throw "Command failed: $Command $($CommandArgs -join ' ') (exit=$exitCode)"
    }
}

function Invoke-McChecked {
    param(
        [Parameter(Mandatory = $true)][object[]]$CommandArgs
    )

    Invoke-Checked -Command $mcCommand -CommandArgs (@("--config-dir", $mcConfigDir) + $CommandArgs)
}

function Invoke-McBestEffort {
    param(
        [Parameter(Mandatory = $true)][object[]]$CommandArgs
    )

    & $mcCommand "--config-dir" $mcConfigDir @CommandArgs
}

function Test-MinioConfig {
    if ($null -eq (Get-Command $mcCommand -ErrorAction SilentlyContinue)) {
        throw "mc is required to upload Windows AgentCafe artifacts."
    }
    if (-not (Test-Path -LiteralPath $mcConfigFile -PathType Leaf)) {
        throw "MinIO mc config file was not found: $mcConfigFile"
    }

    Write-Host "=== verify Windows MinIO config ==="
    Write-Host "MC_CONFIG_DIR=$mcConfigDir"
    Write-Host "MC_CONFIG_FILE=$mcConfigFile"
    Invoke-McChecked -CommandArgs @("ls", "feinian/devops")
}

function Read-VersionEnv {
    param(
        [Parameter(Mandatory = $true)][string]$LiteralPath
    )

    if (-not (Test-Path -LiteralPath $LiteralPath -PathType Leaf)) {
        throw "Missing release metadata: $LiteralPath"
    }

    $values = @{}
    Get-Content -LiteralPath $LiteralPath | ForEach-Object {
        if ($_ -match '^(?:export\s+)?([^=]+)="?([^"]*)"?$') {
            $values[$Matches[1]] = $Matches[2]
        }
    }
    return $values
}

function Get-RelativeArtifactPath {
    param(
        [Parameter(Mandatory = $true)][string]$BasePath,
        [Parameter(Mandatory = $true)][string]$TargetPath
    )

    $baseFull = (Resolve-Path -LiteralPath $BasePath).ProviderPath.Replace("\", "/").TrimEnd("/")
    $targetFull = (Resolve-Path -LiteralPath $TargetPath).ProviderPath.Replace("\", "/")
    $prefix = "$baseFull/"
    if (-not $targetFull.StartsWith($prefix, [System.StringComparison]::OrdinalIgnoreCase)) {
        throw "Target path is outside artifact dir: base=$baseFull target=$targetFull"
    }
    return $targetFull.Substring($prefix.Length)
}

Test-MinioConfig
if ($Action -eq "verify-minio") {
    Write-Host "Windows MinIO config verified."
    exit 0
}

$metadata = Read-VersionEnv -LiteralPath $versionEnvPath
if ([string]::IsNullOrWhiteSpace($metadata["VERSION"])) {
    throw "VERSION is missing from $versionEnvPath"
}
if ([string]::IsNullOrWhiteSpace($metadata["MINIO_RELEASE_ROOT"])) {
    throw "MINIO_RELEASE_ROOT is missing from $versionEnvPath"
}

if ([string]::IsNullOrWhiteSpace($LocalDir)) {
    $LocalDir = Join-Path $repoRoot "artifacts\release\$ArtifactKind"
}

if (-not (Test-Path -LiteralPath $LocalDir -PathType Container)) {
    throw "Artifact directory does not exist: $LocalDir"
}

$files = Get-ChildItem -LiteralPath $LocalDir -File -Recurse | Sort-Object FullName
if (-not $files) {
    throw "Artifact directory is empty: $LocalDir"
}

$localRoot = (Resolve-Path -LiteralPath $LocalDir).ProviderPath
$versionTarget = "$($metadata["MINIO_RELEASE_ROOT"])/$($metadata["VERSION"])/$ArtifactKind/"
$latestTarget = "$($metadata["MINIO_RELEASE_ROOT"])/latest/$ArtifactKind/"

Write-Host "=== upload artifacts to minio ==="
Write-Host "LOCAL_DIR=$localRoot"
Write-Host "VERSION_TARGET=$versionTarget"
Write-Host "LATEST_TARGET=$latestTarget"

Invoke-McBestEffort -CommandArgs @("mb", "-p", $versionTarget) | Out-Host
Invoke-McBestEffort -CommandArgs @("mb", "-p", $latestTarget) | Out-Host
Invoke-McBestEffort -CommandArgs @("rm", "--recursive", "--force", $latestTarget) | Out-Host

foreach ($file in $files) {
    $relativePath = Get-RelativeArtifactPath -BasePath $localRoot -TargetPath $file.FullName
    Invoke-McChecked -CommandArgs @("cp", $file.FullName, "$versionTarget$relativePath")
    Invoke-McChecked -CommandArgs @("cp", $file.FullName, "$latestTarget$relativePath")
}

Write-Host "=== verify uploaded files ==="
foreach ($file in $files) {
    $relativePath = Get-RelativeArtifactPath -BasePath $localRoot -TargetPath $file.FullName
    Invoke-McChecked -CommandArgs @("stat", "$versionTarget$relativePath")
    Invoke-McChecked -CommandArgs @("stat", "$latestTarget$relativePath")
    Write-Host "OK $relativePath"
}
