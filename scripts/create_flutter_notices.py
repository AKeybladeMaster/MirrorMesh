#!/usr/bin/env python3
"""Collect license and notice files for locked Flutter runtime packages."""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path
from urllib.parse import unquote, urlsplit
from urllib.request import url2pathname


NOTICE_PREFIXES = ("license", "licence", "copying", "notice")
NOTICE_SUFFIXES = {"", ".txt", ".md", ".rst", ".text"}
MAX_NOTICE_BYTES = 4 * 1024 * 1024


class NoticeError(ValueError):
    """Raised when the locked runtime graph cannot produce a safe notice set."""


def load_json(path: Path, label: str) -> dict:
    try:
        value = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as exc:
        raise NoticeError(f"cannot read {label} JSON at {path}: {exc}") from exc
    if not isinstance(value, dict):
        raise NoticeError(f"{label} must be a JSON object")
    return value


def package_path(root_uri: str, config_dir: Path) -> Path:
    parsed = urlsplit(root_uri)
    if parsed.scheme == "file":
        if parsed.netloc not in ("", "localhost"):
            raise NoticeError(f"unsupported package file URI host: {parsed.netloc}")
        return Path(url2pathname(unquote(parsed.path))).resolve(strict=True)
    if parsed.scheme:
        raise NoticeError(f"unsupported package URI scheme: {parsed.scheme}")
    return (config_dir / unquote(root_uri)).resolve(strict=True)


def runtime_package_names(graph: dict) -> list[str]:
    packages = graph.get("packages")
    roots = graph.get("roots")
    if not isinstance(packages, list) or not isinstance(roots, list) or not roots:
        raise NoticeError("package graph must contain non-empty roots and packages arrays")

    by_name: dict[str, dict] = {}
    for package in packages:
        if not isinstance(package, dict) or not isinstance(package.get("name"), str):
            raise NoticeError("package graph contains an invalid package entry")
        name = package["name"]
        if name in by_name:
            raise NoticeError(f"package graph contains duplicate package name: {name}")
        by_name[name] = package

    pending: list[str] = []
    for root_name in roots:
        root = by_name.get(root_name)
        if root is None:
            raise NoticeError(f"package graph root is missing: {root_name}")
        dependencies = root.get("dependencies", [])
        if not isinstance(dependencies, list):
            raise NoticeError(f"runtime dependencies are invalid for {root_name}")
        pending.extend(dependencies)

    reached: set[str] = set()
    while pending:
        name = pending.pop()
        if name in reached:
            continue
        package = by_name.get(name)
        if package is None:
            raise NoticeError(f"runtime dependency is missing from package graph: {name}")
        dependencies = package.get("dependencies", [])
        if not isinstance(dependencies, list) or any(not isinstance(item, str) for item in dependencies):
            raise NoticeError(f"dependency list is invalid for {name}")
        reached.add(name)
        pending.extend(dependencies)

    return sorted(reached)


def package_notice_files(root: Path) -> list[Path]:
    if not root.is_dir():
        raise NoticeError(f"package root is not a directory: {root}")

    candidates: list[Path] = []
    for path in root.iterdir():
        lowered = path.name.casefold()
        suffix = path.suffix.casefold()
        if not lowered.startswith(NOTICE_PREFIXES) or suffix not in NOTICE_SUFFIXES:
            continue
        if path.is_symlink():
            raise NoticeError(f"notice file is a symbolic link: {path}")
        if path.is_file():
            candidates.append(path)
    return sorted(candidates, key=lambda path: path.name.casefold())


def read_notice(path: Path) -> str:
    try:
        if path.stat().st_size > MAX_NOTICE_BYTES:
            raise NoticeError(f"notice file exceeds {MAX_NOTICE_BYTES} bytes: {path}")
        text = path.read_text(encoding="utf-8")
    except (OSError, UnicodeDecodeError) as exc:
        raise NoticeError(f"cannot read notice file as UTF-8: {path}: {exc}") from exc
    normalized = text.replace("\r\n", "\n").replace("\r", "\n").strip()
    if not normalized:
        raise NoticeError(f"notice file is empty: {path}")
    return normalized


def generate(package_config_path: Path, package_graph_path: Path) -> tuple[str, int]:
    config = load_json(package_config_path, "package config")
    graph = load_json(package_graph_path, "package graph")
    config_packages = config.get("packages")
    if not isinstance(config_packages, list):
        raise NoticeError("package config must contain a packages array")

    roots: dict[str, Path] = {}
    for item in config_packages:
        if not isinstance(item, dict) or not isinstance(item.get("name"), str):
            raise NoticeError("package config contains an invalid package entry")
        name = item["name"]
        uri = item.get("rootUri")
        if not isinstance(uri, str) or name in roots:
            raise NoticeError(f"invalid or duplicate package config entry: {name}")
        roots[name] = package_path(uri, package_config_path.parent)

    package_entries = {item["name"]: item for item in graph.get("packages", []) if isinstance(item, dict)}
    names = runtime_package_names(graph)
    sections = [
        "MirrorMesh Flutter runtime dependency notices",
        "",
        "Generated from the locked Flutter package graph. Each section contains the package's root-level license and notice files.",
        "This Flutter package inventory does not cover native SDK, Gradle, Apple, codec, or other platform dependencies.",
        "",
    ]

    for name in names:
        root = roots.get(name)
        if root is None:
            raise NoticeError(f"runtime dependency is missing from package config: {name}")
        entry = package_entries.get(name)
        if entry is None or not isinstance(entry.get("version"), str):
            raise NoticeError(f"runtime dependency has no exact version in package graph: {name}")
        license_paths = package_notice_files(root)
        if not license_paths:
            raise NoticeError(f"no root-level license or notice file found for {name} {entry['version']}")

        sections.extend((f"## {name} {entry['version']}", ""))
        for path in license_paths:
            sections.extend((f"### {path.name}", "", read_notice(path), ""))

    return "\n".join(sections).rstrip() + "\n", len(names)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--package-config", type=Path, required=True, help=".dart_tool/package_config.json")
    parser.add_argument("--package-graph", type=Path, required=True, help=".dart_tool/package_graph.json")
    parser.add_argument("--output", type=Path, required=True, help="destination UTF-8 text file")
    args = parser.parse_args()

    try:
        content, package_count = generate(args.package_config, args.package_graph)
        args.output.parent.mkdir(parents=True, exist_ok=True)
        args.output.write_text(content, encoding="utf-8", newline="\n")
    except (NoticeError, OSError) as exc:
        print(f"flutter notice generation failed: {exc}", file=sys.stderr)
        return 2

    print(f"Generated {package_count} Flutter runtime package notices: {args.output}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
