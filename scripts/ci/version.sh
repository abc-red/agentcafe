#!/usr/bin/env bash

set -euo pipefail

VERSION_ENV="${VERSION_ENV:-.drone/version.env}"
mkdir -p "$(dirname "$VERSION_ENV")"

drone_tag="${DRONE_TAG:-}"
drone_build_number="${DRONE_BUILD_NUMBER:-}"

if [ -n "$drone_tag" ]; then
  version="$drone_tag"
else
  build_id="$drone_build_number"
  if [ -z "$build_id" ]; then
    build_id="$(date -u '+%Y%m%d%H%M%S')"
  fi
  version="v0.0.0-dev.$build_id"
fi

cat > "$VERSION_ENV" <<EOF
export VERSION="$version"
export MINIO_RELEASE_ROOT="feinian/devops/agentcafe"
EOF

echo "=== release metadata ==="
cat "$VERSION_ENV"
