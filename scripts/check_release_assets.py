#!/usr/bin/env python3
"""Validate release staging directories and installer source inputs."""

from __future__ import annotations

import argparse
import glob
import sys
import xml.etree.ElementTree as ET
from pathlib import Path


FORBIDDEN_SUFFIXES = {
    ".nes",
    ".fds",
    ".unf",
    ".unif",
    ".ips",
    ".bps",
    ".sav",
    ".srm",
    ".png",
    ".jpg",
    ".jpeg",
    ".gif",
    ".bmp",
    ".mp3",
    ".wav",
    ".flac",
    ".ogg",
    ".mp4",
    ".mov",
    ".mkv",
    ".pdf",
}

EXPECTED_WIX_SOURCES = {
    "wix/license.rtf",
    "$(var.cargotargetbindir)/oxidenes.exe",
}


def fail(message: str) -> None:
    print(f"release asset check failed: {message}", file=sys.stderr)
    raise SystemExit(1)


def normalize_source(source: str) -> str:
    return source.replace("\\", "/").lower()


def check_forbidden_suffix(path: Path, label: str) -> None:
    if path.suffix.lower() in FORBIDDEN_SUFFIXES:
        fail(f"{label} has forbidden release suffix: {path}")


def check_asset_dir(asset_dir: Path, expected_asset_name: str) -> None:
    if not asset_dir.is_dir():
        fail(f"asset directory does not exist: {asset_dir}")

    files = sorted(path for path in asset_dir.iterdir() if path.is_file())
    names = [path.name for path in files]
    if names != [expected_asset_name]:
        fail(f"expected only {expected_asset_name!r} in {asset_dir}, found {names!r}")

    asset = files[0]
    if asset.stat().st_size == 0:
        fail(f"asset is empty: {asset}")
    check_forbidden_suffix(asset, "staged asset")
    print(f"staged asset OK: {asset}")


def check_installer_glob(installer_glob: str) -> None:
    installers = sorted(Path(path) for path in glob.glob(installer_glob))
    if not installers:
        fail(f"installer glob matched no files: {installer_glob}")

    for installer in installers:
        if installer.suffix.lower() != ".msi":
            fail(f"installer artifact is not an MSI: {installer}")
        if installer.stat().st_size == 0:
            fail(f"installer artifact is empty: {installer}")
        print(f"installer artifact OK: {installer}")


def check_wix_source(wix_source: Path) -> None:
    if not wix_source.is_file():
        fail(f"WiX source does not exist: {wix_source}")

    root = ET.parse(wix_source).getroot()
    active_sources = []
    for element in root.iter():
        if element.tag.endswith("File"):
            source = element.attrib.get("Source")
            if source:
                active_sources.append(normalize_source(source))

    if not active_sources:
        fail(f"WiX source has no active File Source entries: {wix_source}")

    unexpected = sorted(set(active_sources) - EXPECTED_WIX_SOURCES)
    if unexpected:
        fail(f"unexpected WiX package inputs: {unexpected!r}")

    for source in active_sources:
        check_forbidden_suffix(Path(source), "WiX package input")
    print(f"WiX package inputs OK: {', '.join(active_sources)}")


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--asset-dir", type=Path, required=True)
    parser.add_argument("--expected-asset-name", required=True)
    parser.add_argument("--installer-glob")
    parser.add_argument("--wix-source", type=Path)
    args = parser.parse_args()

    check_asset_dir(args.asset_dir, args.expected_asset_name)

    if args.installer_glob:
        check_installer_glob(args.installer_glob)
    if args.wix_source:
        check_wix_source(args.wix_source)

    print("release asset checks passed")


if __name__ == "__main__":
    main()
