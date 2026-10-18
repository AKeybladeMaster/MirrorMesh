#!/usr/bin/env python3
"""Create deterministic checksums and a manifest for staged release artifacts."""

from __future__ import annotations

import argparse
import hashlib
import json
import re
import sys
from pathlib import Path, PurePosixPath
from typing import Any


COMMIT_RE = re.compile(r"^(?:[0-9a-f]{40}|[0-9a-f]{64})$", re.IGNORECASE)
PLATFORM_RE = re.compile(r"^[a-z0-9]+(?:-[a-z0-9]+)+$")
PLACEHOLDER_RE = re.compile(r"<[^>]+>|replace[-_ ]?with|todo", re.IGNORECASE)
LATEST_RE = re.compile(r"\blatest\b|\bcurrent\b", re.IGNORECASE)
CHUNK_SIZE = 1024 * 1024


class MetadataError(ValueError):
    """Raised when release metadata is incomplete or unsafe to publish."""


def require_exact_text(label: str, value: Any) -> str:
    if not isinstance(value, str) or not value.strip():
        raise MetadataError(f"{label} must be a non-empty string")
    value = value.strip()
    if PLACEHOLDER_RE.search(value):
        raise MetadataError(f"{label} still contains a template placeholder")
    if LATEST_RE.search(value):
        raise MetadataError(f"{label} must record an exact version, not latest/current")
    return value


def load_context(path: Path) -> dict[str, Any]:
    try:
        context = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as exc:
        raise MetadataError(f"cannot read release context JSON: {exc}") from exc
    if not isinstance(context, dict):
        raise MetadataError("release context must be a JSON object")

    require_exact_text("release_tag", context.get("release_tag"))
    revision = require_exact_text("source_revision", context.get("source_revision"))
    if not COMMIT_RE.fullmatch(revision):
        raise MetadataError("source_revision must be a full 40- or 64-character Git commit ID")
    platform = require_exact_text("platform", context.get("platform"))
    if not PLATFORM_RE.fullmatch(platform):
        raise MetadataError("platform must be a lowercase, hyphenated track ID")
    require_exact_text("runner_image", context.get("runner_image"))

    sdk_versions = context.get("sdk_versions")
    if not isinstance(sdk_versions, dict) or not sdk_versions:
        raise MetadataError("sdk_versions must contain the exact tool versions used")
    for name, version in sdk_versions.items():
        require_exact_text(f"sdk_versions.{name}", version)

    commands = context.get("build_commands")
    if not isinstance(commands, list) or not commands:
        raise MetadataError("build_commands must list the successful build commands")
    for index, command in enumerate(commands):
        require_exact_text(f"build_commands[{index}]", command)

    reproducibility = context.get("reproducibility")
    if not isinstance(reproducibility, dict):
        raise MetadataError("reproducibility must be an object")
    status = require_exact_text("reproducibility.status", reproducibility.get("status"))
    if status not in {"not_verified", "verified"}:
        raise MetadataError("reproducibility.status must be not_verified or verified")
    require_exact_text("reproducibility.notes", reproducibility.get("notes"))

    for field in ("sbom_file", "third_party_notices_file"):
        file_name = require_exact_text(field, context.get(field))
        relative = PurePosixPath(file_name)
        if (
            relative.is_absolute()
            or ".." in relative.parts
            or "." in relative.parts
            or "\\" in file_name
            or ":" in file_name
        ):
            raise MetadataError(f"{field} must be a path relative to the artifact directory")

    return context


def digest_file(path: Path) -> str:
    hasher = hashlib.sha256()
    with path.open("rb") as source:
        for chunk in iter(lambda: source.read(CHUNK_SIZE), b""):
            hasher.update(chunk)
    return hasher.hexdigest()


def staged_artifacts(artifact_dir: Path) -> list[dict[str, Any]]:
    artifacts: list[dict[str, Any]] = []
    for path in sorted(artifact_dir.rglob("*")):
        if path.is_symlink():
            raise MetadataError(f"symbolic links are not allowed in the artifact directory: {path}")
        if not path.is_file():
            continue
        relative_name = path.relative_to(artifact_dir).as_posix()
        size = path.stat().st_size
        if size == 0:
            raise MetadataError(f"empty release artifact: {relative_name}")
        artifacts.append(
            {
                "path": relative_name,
                "size_bytes": size,
                "sha256": digest_file(path),
            }
        )
    if not artifacts:
        raise MetadataError("artifact directory contains no files")
    return artifacts


def validate_required_artifact(context: dict[str, Any], artifacts: list[dict[str, Any]]) -> None:
    names = {artifact["path"] for artifact in artifacts}
    for field in ("sbom_file", "third_party_notices_file"):
        required_name = PurePosixPath(context[field]).as_posix()
        if required_name not in names:
            raise MetadataError(f"required {field} is missing from staged artifacts: {required_name}")
        staged_path = next(item for item in artifacts if item["path"] == required_name)
        artifact_root = context["_artifact_root"]
        path = artifact_root.joinpath(*PurePosixPath(required_name).parts)
        if field == "sbom_file":
            try:
                sbom = json.loads(path.read_text(encoding="utf-8"))
            except (OSError, json.JSONDecodeError) as exc:
                raise MetadataError(f"SBOM must be valid JSON: {exc}") from exc
            if not isinstance(sbom, dict) or not (
                sbom.get("bomFormat") == "CycloneDX"
                or (isinstance(sbom.get("spdxVersion"), str) and sbom["spdxVersion"].startswith("SPDX-"))
            ):
                raise MetadataError("SBOM JSON must identify CycloneDX or SPDX format")
        else:
            try:
                notice_text = path.read_text(encoding="utf-8").strip()
            except (OSError, UnicodeDecodeError) as exc:
                raise MetadataError(f"third-party notices must be readable UTF-8 text: {exc}") from exc
            if not notice_text or staged_path["size_bytes"] == 0:
                raise MetadataError("third-party notices file must not be empty")


def write_deterministic(path: Path, content: str) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(content, encoding="utf-8", newline="\n")


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--artifacts", required=True, type=Path, help="directory containing staged release files")
    parser.add_argument("--context", required=True, type=Path, help="filled release context JSON")
    parser.add_argument("--output", required=True, type=Path, help="separate directory for manifest and checksums")
    args = parser.parse_args()

    try:
        artifact_dir = args.artifacts.resolve(strict=True)
        output_dir = args.output.resolve(strict=False)
        if not artifact_dir.is_dir():
            raise MetadataError("--artifacts must be a directory")
        try:
            output_dir.relative_to(artifact_dir)
        except ValueError:
            pass
        else:
            raise MetadataError("--output must be outside the artifact directory")

        context = load_context(args.context)
        artifacts = staged_artifacts(artifact_dir)
        context["_artifact_root"] = artifact_dir
        validate_required_artifact(context, artifacts)
        context.pop("_artifact_root")

        checksum_text = "".join(f"{item['sha256']}  {item['path']}\n" for item in artifacts)
        manifest = {
            "schema_version": 1,
            **context,
            "artifacts": artifacts,
            "checksums_file": "SHA256SUMS",
        }
        write_deterministic(output_dir / "SHA256SUMS", checksum_text)
        write_deterministic(
            output_dir / "release-manifest.json",
            json.dumps(manifest, indent=2, sort_keys=True) + "\n",
        )
    except (MetadataError, OSError) as exc:
        print(f"release metadata error: {exc}", file=sys.stderr)
        return 2

    print(f"Wrote checksums and manifest for {len(artifacts)} artifacts to {output_dir}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
