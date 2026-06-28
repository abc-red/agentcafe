#!/usr/bin/env bash

set -euo pipefail

if [ "$#" -ne 2 ]; then
  echo "Usage: $0 <local-artifact-dir> <artifact-kind>" >&2
  exit 1
fi

local_dir="$1"
artifact_kind="$2"

VERSION_ENV="${VERSION_ENV:-.drone/version.env}"
if [ ! -f "$VERSION_ENV" ]; then
  bash scripts/ci/version.sh
fi
. "$VERSION_ENV"

if [ ! -d "$local_dir" ]; then
  echo "ERROR: artifact directory does not exist: $local_dir" >&2
  exit 1
fi

if ! find "$local_dir" -type f | grep -q .; then
  echo "ERROR: artifact directory is empty: $local_dir" >&2
  exit 1
fi

version_target="${MINIO_RELEASE_ROOT}/${VERSION}/${artifact_kind}/"
latest_target="${MINIO_RELEASE_ROOT}/latest/${artifact_kind}/"

verify_remote_tree() {
  target="$1"
  echo "=== verify uploaded files: $target ==="
  find "$local_dir" -type f | sort | while IFS= read -r local_file; do
    relative_path="${local_file#"$local_dir"/}"
    mc stat "${target}${relative_path}" >/dev/null
    echo "OK ${target}${relative_path}"
  done
}

echo "=== upload artifacts to minio ==="
echo "LOCAL_DIR=$local_dir"
echo "VERSION_TARGET=$version_target"
echo "LATEST_TARGET=$latest_target"

mc mb -p "$version_target" || true
mc mb -p "$latest_target" || true
mc rm --recursive --force "$latest_target" || true
mc cp --recursive "$local_dir/" "$version_target"
mc cp --recursive "$local_dir/" "$latest_target"

echo "=== uploaded files ==="
mc ls "$version_target"
verify_remote_tree "$version_target"
verify_remote_tree "$latest_target"
