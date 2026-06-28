param(
    [string]$LocalDir,
    [string]$ArtifactKind = "windows"
)

$ErrorActionPreference = "Stop"

$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot "..\..")).Path
$versionEnvPath = Join-Path $repoRoot ".drone\version.env"

if (-not (Test-Path $versionEnvPath)) {
    throw "Missing release metadata: $versionEnvPath"
}

$metadata = @{}
Get-Content $versionEnvPath | ForEach-Object {
    if ($_ -match '^(?:export\s+)?([^=]+)=(.*)$') {
        $metadata[$Matches[1]] = $Matches[2].Trim('"')
    }
}

if (-not $metadata.ContainsKey("VERSION")) {
    throw "VERSION is missing from $versionEnvPath"
}

if (-not $metadata.ContainsKey("MINIO_RELEASE_ROOT")) {
    throw "MINIO_RELEASE_ROOT is missing from $versionEnvPath"
}

if (-not $LocalDir) {
    $LocalDir = Join-Path $repoRoot "artifacts\release\$ArtifactKind"
}

if (-not (Test-Path $LocalDir)) {
    Write-Host "SKIP: artifact directory does not exist: $LocalDir"
    exit 0
}

$files = Get-ChildItem -Path $LocalDir -File -Recurse | Sort-Object FullName
if (-not $files) {
    Write-Host "SKIP: artifact directory is empty: $LocalDir"
    exit 0
}

$localRoot = (Resolve-Path $LocalDir).Path
$versionTarget = "$($metadata.MINIO_RELEASE_ROOT)/$($metadata.VERSION)/$ArtifactKind/"
$latestTarget = "$($metadata.MINIO_RELEASE_ROOT)/latest/$ArtifactKind/"

function Get-RelativeArtifactPath {
    param(
        [string]$FilePath
    )

    $root = $localRoot
    if (-not $root.EndsWith([System.IO.Path]::DirectorySeparatorChar)) {
        $root = "$root$([System.IO.Path]::DirectorySeparatorChar)"
    }

    $rootUri = New-Object System.Uri($root)
    $fileUri = New-Object System.Uri($FilePath)
    return [System.Uri]::UnescapeDataString($rootUri.MakeRelativeUri($fileUri).ToString()).Replace('\', '/')
}

function Copy-TreeToMinio {
    param(
        [string]$Target
    )

    foreach ($file in $files) {
        $relativePath = Get-RelativeArtifactPath -FilePath $file.FullName
        mc cp $file.FullName "$Target$relativePath"
    }
}

function Verify-RemoteTree {
    param(
        [string]$Target
    )

    Write-Host "=== verify uploaded files: $Target ==="
    foreach ($file in $files) {
        $relativePath = Get-RelativeArtifactPath -FilePath $file.FullName
        mc stat "$Target$relativePath" | Out-Null
        Write-Host "OK $Target$relativePath"
    }
}

Write-Host "=== upload artifacts to minio ==="
Write-Host "LOCAL_DIR=$localRoot"
Write-Host "VERSION_TARGET=$versionTarget"
Write-Host "LATEST_TARGET=$latestTarget"

mc mb -p $versionTarget 2>$null
mc mb -p $latestTarget 2>$null
Copy-TreeToMinio -Target $versionTarget
Copy-TreeToMinio -Target $latestTarget

Write-Host "=== uploaded files ==="
mc ls $versionTarget
Verify-RemoteTree -Target $versionTarget
Verify-RemoteTree -Target $latestTarget
