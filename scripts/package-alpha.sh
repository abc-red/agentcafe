#!/usr/bin/env bash
set -euo pipefail

version="${1:-v0.2.0-alpha.1}"
repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
dist_dir="$repo_root/dist/alpha/$version"

mkdir -p "$dist_dir"
cd "$repo_root"

cargo build -p agentcafe-sidecar --release

case "$(uname -s)" in
  Darwin)
    swift build --package-path apps/macos -c release
    package_dir="$dist_dir/AgentCafe-macos-$version"
    rm -rf "$package_dir"
    mkdir -p "$package_dir/bin"
    cp "apps/macos/.build/release/AgentCafeMac" "$package_dir/AgentCafeMac"
    cp "target/release/agentcafe-sidecar" "$package_dir/bin/agentcafe-sidecar"
    cat > "$package_dir/README.txt" <<EOF
Agent Cafe $version macOS controlled Alpha

Run:
  AGENTCAFE_SIDECAR="\$(pwd)/bin/agentcafe-sidecar" ./AgentCafeMac

This Alpha is read-only. It runs ipc.handshake and doctor.run only.
MVP2 write, snapshot, restore, MCP test, Hook, Plugin, and Skill write actions are disabled.
EOF
    (cd "$dist_dir" && zip -qr "AgentCafe-macos-$version.zip" "AgentCafe-macos-$version")
    shasum -a 256 "$dist_dir/AgentCafe-macos-$version.zip" > "$dist_dir/AgentCafe-macos-$version.zip.sha256"
    ;;
  MINGW*|MSYS*|CYGWIN*|Windows_NT)
    dotnet publish apps/windows-wpf/AgentCafe.Windows.csproj -c Release -o "$dist_dir/windows-publish"
    package_dir="$dist_dir/AgentCafe-windows-$version"
    rm -rf "$package_dir"
    mkdir -p "$package_dir/bin"
    cp -R "$dist_dir/windows-publish/." "$package_dir/"
    cp "target/release/agentcafe-sidecar.exe" "$package_dir/bin/agentcafe-sidecar.exe"
    cat > "$package_dir/README.txt" <<EOF
Agent Cafe $version Windows controlled Alpha

PowerShell run:
  \$env:AGENTCAFE_SIDECAR="\$PWD\\bin\\agentcafe-sidecar.exe"
  .\\AgentCafe.Windows.exe

This Alpha is read-only. It runs ipc.handshake and doctor.run only.
MVP2 write, snapshot, restore, MCP test, Hook, Plugin, and Skill write actions are disabled.
EOF
    (cd "$dist_dir" && powershell -NoProfile -Command "Compress-Archive -Path 'AgentCafe-windows-$version' -DestinationPath 'AgentCafe-windows-$version.zip' -Force")
    powershell -NoProfile -Command "(Get-FileHash '$dist_dir/AgentCafe-windows-$version.zip' -Algorithm SHA256).Hash + '  AgentCafe-windows-$version.zip'" > "$dist_dir/AgentCafe-windows-$version.zip.sha256"
    ;;
  *)
    echo "Unsupported packaging host: $(uname -s)" >&2
    exit 1
    ;;
esac

echo "Alpha package written to $dist_dir"
