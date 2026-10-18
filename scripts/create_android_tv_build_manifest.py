#!/usr/bin/env python3
"""Capture exact Android TV CI tool versions and built-package digests."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import re
import subprocess
import sys
from pathlib import Path
from typing import Any


REVISION_RE = re.compile(r"^(?:[0-9a-f]{40}|[0-9a-f]{64})$", re.IGNORECASE)
GRADLE_RE = re.compile(r"\bGradle\s+([0-9]+(?:\.[0-9]+){1,3}(?:[-+][A-Za-z0-9.-]+)?)")
PLUGIN_RE = re.compile(
    r'id\("com\.android\.application"\)\s+version\s+"([^"]+)"'
)
PACKAGE_ID_RE = re.compile(r"^[A-Za-z0-9_.-]+(?:;[A-Za-z0-9_.-]+)*$")
EXPECTED_GRADLE = "9.6.0"
EXPECTED_ANDROID_GRADLE_PLUGIN = "9.4.0"


class ManifestError(ValueError):
    """Raised when exact build evidence is missing or inconsistent."""


def run_output(command: list[str]) -> str:
    try:
        result = subprocess.run(command, capture_output=True, text=True, check=False)
    except OSError as exc:
        raise ManifestError(f"cannot run {' '.join(command)}: {exc}") from exc
    output = "\n".join(value for value in (result.stdout, result.stderr) if value).strip()
    if result.returncode != 0:
        raise ManifestError(f"{' '.join(command)} failed with exit {result.returncode}: {output}")
    if not output:
        raise ManifestError(f"{' '.join(command)} returned no version information")
    return output


def first_line(output: str, label: str) -> str:
    for line in output.splitlines():
        if line.strip():
            return line.strip()
    raise ManifestError(f"{label} returned no non-empty version line")


def run_version(command: list[str], pattern: re.Pattern[str], label: str) -> str:
    output = run_output(command)
    match = pattern.search(output)
    if not match:
        raise ManifestError(f"could not identify exact {label} version in command output")
    return match.group(1)


def require_environment(name: str) -> str:
    value = os.environ.get(name, "").strip()
    if not value:
        raise ManifestError(f"required hosted runner variable {name} is missing")
    if "latest" in value.casefold() or "current" in value.casefold():
        raise ManifestError(f"{name} does not identify an exact version: {value}")
    return value


def android_sdk_packages() -> dict[str, str]:
    output = run_output(["sdkmanager", "--list_installed"])
    packages: dict[str, str] = {}
    for line in output.splitlines():
        fields = line.split("|")
        if len(fields) < 2:
            continue
        package_id = fields[0].strip()
        version = fields[1].strip()
        if PACKAGE_ID_RE.fullmatch(package_id) and re.match(r"^\d", version):
            packages[package_id] = version

    required = {"platforms;android-37", "build-tools;36.0.0"}
    missing = sorted(required - packages.keys())
    if missing:
        raise ManifestError(f"required Android SDK packages are not installed: {', '.join(missing)}")
    return dict(sorted(packages.items()))


def file_digest(path: Path, root: Path) -> dict[str, Any]:
    try:
        resolved = path.resolve(strict=True)
    except OSError as exc:
        raise ManifestError(f"artifact is missing: {path}: {exc}") from exc
    if path.is_symlink() or not resolved.is_file():
        raise ManifestError(f"artifact must be a regular, non-symlink file: {path}")
    try:
        relative = resolved.relative_to(root)
    except ValueError as exc:
        raise ManifestError(f"artifact must be inside the workspace: {path}") from exc
    size = resolved.stat().st_size
    if size == 0:
        raise ManifestError(f"artifact is empty: {path}")

    digest = hashlib.sha256()
    with resolved.open("rb") as source:
        for chunk in iter(lambda: source.read(1024 * 1024), b""):
            digest.update(chunk)
    return {"path": relative.as_posix(), "size_bytes": size, "sha256": digest.hexdigest()}


def load_cyclonedx(path: Path) -> dict[str, Any]:
    try:
        sbom = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as exc:
        raise ManifestError(f"cannot read package SBOM JSON: {exc}") from exc
    if not isinstance(sbom, dict) or sbom.get("bomFormat") != "CycloneDX":
        raise ManifestError("package SBOM must be JSON in CycloneDX format")
    return sbom


def create_manifest(apk: Path, sbom_path: Path, gradle_config: Path) -> dict[str, Any]:
    root = Path.cwd().resolve(strict=True)
    revision = require_environment("GITHUB_SHA")
    if not REVISION_RE.fullmatch(revision):
        raise ManifestError("GITHUB_SHA must be a full 40- or 64-character commit ID")

    try:
        gradle_text = gradle_config.read_text(encoding="utf-8")
    except OSError as exc:
        raise ManifestError(f"cannot read Android build configuration: {exc}") from exc
    plugin_match = PLUGIN_RE.search(gradle_text)
    if not plugin_match:
        raise ManifestError("could not find a pinned Android Gradle Plugin version")
    if plugin_match.group(1) != EXPECTED_ANDROID_GRADLE_PLUGIN:
        raise ManifestError(
            f"Android Gradle Plugin version changed; update the evidence gate: {plugin_match.group(1)}"
        )

    java_version = first_line(run_output(["java", "--version"]), "Java")
    if not re.search(r"\b(?:openjdk|java)(?:\s+version)?\s+\"?17(?:[.\s\"+-])", java_version, re.IGNORECASE):
        raise ManifestError(f"expected JDK 17, observed: {java_version}")
    gradle_version = run_version(["gradle", "--version"], GRADLE_RE, "Gradle")
    if gradle_version != EXPECTED_GRADLE:
        raise ManifestError(f"expected Gradle {EXPECTED_GRADLE}, observed: {gradle_version}")
    sdkmanager_version = first_line(run_output(["sdkmanager", "--version"]), "sdkmanager")
    sdk_packages = android_sdk_packages()

    if sbom_path.is_symlink():
        raise ManifestError("package SBOM must be a regular, non-symlink file")
    try:
        sbom_file = sbom_path.resolve(strict=True)
    except OSError as exc:
        raise ManifestError(f"package SBOM is missing: {exc}") from exc
    if not sbom_file.is_file():
        raise ManifestError("package SBOM must be a regular, non-symlink file")
    load_cyclonedx(sbom_file)

    return {
        "schema_version": 1,
        "purpose": "android-tv-debug-ci-build-evidence",
        "platform": "android-tv",
        "source_revision": revision,
        "runner_image": {
            "os": require_environment("ImageOS"),
            "version": require_environment("ImageVersion"),
        },
        "sdk_versions": {
            "java_runtime": java_version,
            "gradle": gradle_version,
            "android_gradle_plugin": plugin_match.group(1),
            "sdkmanager": sdkmanager_version,
            "android_sdk_packages": sdk_packages,
        },
        "build_commands": ["gradle :app:assembleDebug"],
        "artifacts": {
            "apk": file_digest(apk, root),
            "sbom": file_digest(sbom_path, root),
        },
        "release_status": "debug_build_only_not_release_eligible",
    }


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--apk", required=True, type=Path, help="built Android TV APK")
    parser.add_argument("--sbom", required=True, type=Path, help="CycloneDX JSON generated from the APK")
    parser.add_argument("--gradle-config", required=True, type=Path, help="Android TV root build.gradle.kts")
    parser.add_argument("--output", required=True, type=Path, help="build manifest destination")
    args = parser.parse_args()

    try:
        manifest = create_manifest(args.apk, args.sbom, args.gradle_config)
        args.output.parent.mkdir(parents=True, exist_ok=True)
        args.output.write_text(
            json.dumps(manifest, ensure_ascii=False, indent=2, sort_keys=True) + "\n",
            encoding="utf-8",
            newline="\n",
        )
    except (ManifestError, OSError, ValueError) as exc:
        print(f"Android TV build manifest creation failed: {exc}", file=sys.stderr)
        return 2

    print(f"Wrote Android TV debug build evidence: {args.output}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
