#!/usr/bin/env bash
# Download the full experiment datasets from the Zenodo archive (DOI
# 10.5281/zenodo.23295261) and unpack them into ./experiments.
#
# The repository only keeps one small sample run (experiments/azure_d4s_v6) plus
# the JSON analysis outputs; the raw CSVs for the other runs live on Zenodo.
# Run this before the analysis scripts in README.md ("Reproducing the paper").
set -euo pipefail

URL="https://zenodo.org/api/records/23295261/files/Saksham932007/RustRedis-v1.0-paper.zip/content"
MD5="f3d039e40cf9b877b1b3f7f2dcc3c91e"

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT

echo "Downloading archive from Zenodo..."
curl -fSL --retry 3 -o "$tmp/archive.zip" "$URL"

echo "Verifying checksum..."
actual="$(md5sum "$tmp/archive.zip" | cut -d' ' -f1)"
if [ "$actual" != "$MD5" ]; then
    echo "Checksum mismatch: expected $MD5, got $actual" >&2
    exit 1
fi

echo "Unpacking experiments/ ..."
unzip -q "$tmp/archive.zip" '*/experiments/*' -d "$tmp/unpacked"
mkdir -p "$repo_root/experiments"
cp -r "$tmp"/unpacked/*/experiments/. "$repo_root/experiments/"

echo "Done. Datasets are in $repo_root/experiments"
