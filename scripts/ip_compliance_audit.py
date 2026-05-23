#!/usr/bin/env python3
"""Audit tracked repository files for public-distribution IP hazards."""

from __future__ import annotations

import re
import subprocess
import sys
from pathlib import Path


FORBIDDEN_TRACKED_SUFFIXES = {
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
    ".webp",
    ".ico",
    ".icns",
    ".mp3",
    ".wav",
    ".flac",
    ".ogg",
    ".mp4",
    ".mov",
    ".mkv",
    ".pdf",
    ".zip",
    ".7z",
    ".rar",
}

TEXT_SUFFIXES = {
    "",
    ".bat",
    ".cfg",
    ".css",
    ".html",
    ".js",
    ".json",
    ".lock",
    ".lua",
    ".md",
    ".ps1",
    ".py",
    ".rs",
    ".rtf",
    ".toml",
    ".txt",
    ".wxs",
    ".xml",
    ".yml",
}

PROBLEM_TEXT_PATTERNS = [
    (
        re.compile(r"\bdownload\s+(?:a\s+|any\s+|the\s+)?roms?\b", re.IGNORECASE),
        "documentation appears to encourage downloading ROMs",
    ),
    (
        re.compile(r"\broms?\s+download\b", re.IGNORECASE),
        "documentation appears to point users to unauthorized ROM sources",
    ),
    (
        re.compile(r"\bfree\s+roms?\b", re.IGNORECASE),
        "documentation appears to point users to unauthorized no-cost ROM sources",
    ),
]


class AuditFailure(Exception):
    pass


def tracked_files() -> list[Path]:
    result = subprocess.run(
        ["git", "ls-files", "-z"],
        check=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
    )
    return [Path(raw.decode("utf-8")) for raw in result.stdout.split(b"\0") if raw]


def fail(message: str) -> None:
    raise AuditFailure(message)


def check_suffixes(paths: list[Path]) -> None:
    offenders = [path.as_posix() for path in paths if path.suffix.lower() in FORBIDDEN_TRACKED_SUFFIXES]
    if offenders:
        fail("forbidden tracked asset type(s): " + ", ".join(offenders))


def check_text_patterns(paths: list[Path]) -> None:
    for path in paths:
        if path.suffix.lower() not in TEXT_SUFFIXES:
            continue
        try:
            text = path.read_text(encoding="utf-8")
        except UnicodeDecodeError:
            text = path.read_text(encoding="utf-8", errors="ignore")
        for regex, reason in PROBLEM_TEXT_PATTERNS:
            for match in regex.finditer(text):
                line = text.count("\n", 0, match.start()) + 1
                fail(f"{reason}: {path.as_posix()}:{line}")


def main() -> None:
    paths = tracked_files()
    check_suffixes(paths)
    check_text_patterns(paths)
    print(f"IP compliance audit passed ({len(paths)} tracked files checked)")


if __name__ == "__main__":
    try:
        main()
    except AuditFailure as exc:
        print(f"IP compliance audit failed: {exc}", file=sys.stderr)
        raise SystemExit(1)
