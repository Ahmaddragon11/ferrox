#!/usr/bin/env python3
# SPDX-License-Identifier: MIT OR Apache-2.0
"""Helper for the Ferrox release pipeline.

Subcommands
-----------
resolve   print the version and channel that are stored in ``version.toml``
bump      rewrite ``version.toml`` and ``Cargo.toml`` with the next version
sign      sign every release asset with the Ed25519 release key
manifest  write the update manifest that Ferrox uses for self-updates

The script only relies on the Python standard library so it can run on every
GitHub runner without any extra setup.
"""

from __future__ import annotations

import argparse
import datetime as dt
import hashlib
import json
import os
import pathlib
import re
import subprocess
import sys

ROOT = pathlib.Path(__file__).resolve().parent.parent
VERSION_TOML = ROOT / "version.toml"
CARGO_TOML = ROOT / "Cargo.toml"

#: Platform identifier used by ``ferrox-updater`` mapped to the archive name.
ARCHIVES = {
    "linux-x86_64": "ferrox-linux-x86_64.tar.gz",
    "windows-x86_64": "ferrox-windows-x86_64.zip",
}


# ---------------------------------------------------------------------------
# helpers
# ---------------------------------------------------------------------------


def read_version_toml() -> dict[str, str]:
    """Parses the simple ``key = "value"`` format of ``version.toml``."""
    values: dict[str, str] = {}
    for line in VERSION_TOML.read_text(encoding="utf-8").splitlines():
        stripped = line.strip()
        if not stripped or stripped.startswith("#") or "=" not in stripped:
            continue
        key, _, raw = stripped.partition("=")
        values[key.strip()] = raw.strip().strip('"')
    return values


def replace_version(text: str, version: str) -> str:
    return re.sub(
        r'(?m)^version\s*=\s*".*"$',
        f'version = "{version}"',
        text,
        count=1,
    )


def set_version_toml(version: str) -> None:
    text = VERSION_TOML.read_text(encoding="utf-8")
    VERSION_TOML.write_text(replace_version(text, version), encoding="utf-8")


def set_cargo_version(version: str) -> None:
    text = CARGO_TOML.read_text(encoding="utf-8")
    CARGO_TOML.write_text(replace_version(text, version), encoding="utf-8")


def bump_version(version: str, kind: str) -> str:
    """Increments a semantic version."""
    if kind == "none":
        return version
    parts = version.lstrip("v").split(".")
    if len(parts) != 3 or not all(part.isdigit() for part in parts):
        raise SystemExit(f"cannot bump the version {version!r}")
    major, minor, patch = (int(part) for part in parts)
    if kind == "major":
        major, minor, patch = major + 1, 0, 0
    elif kind == "minor":
        minor, patch = minor + 1, 0
    elif kind == "patch":
        patch += 1
    else:
        raise SystemExit(f"unknown bump strategy {kind!r}")
    return f"{major}.{minor}.{patch}"


def sha256_of(path: pathlib.Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def repository_slug() -> str:
    slug = os.environ.get("GITHUB_REPOSITORY", "").strip()
    if not slug:
        raise SystemExit("GITHUB_REPOSITORY is not set")
    return slug


def download_url(slug: str, version: str, name: str) -> str:
    return f"https://github.com/{slug}/releases/download/v{version}/{name}"


# ---------------------------------------------------------------------------
# subcommands
# ---------------------------------------------------------------------------


def cmd_resolve(args: argparse.Namespace) -> int:
    values = read_version_toml()
    version = (args.version or values.get("version", "")).lstrip("v")
    if not version:
        raise SystemExit("no version found in version.toml")
    channel = values.get("channel", "stable")
    print(f"version={version}")
    print(f"channel={channel}")
    return 0


def cmd_bump(args: argparse.Namespace) -> int:
    current = read_version_toml().get("version", "0.0.0")
    nxt = bump_version(current, args.bump)
    set_version_toml(nxt)
    set_cargo_version(nxt)
    print(f"version={nxt}")
    return 0


def cmd_sign(args: argparse.Namespace) -> int:
    dist = pathlib.Path(args.dist)
    slug = repository_slug()
    version = args.version.lstrip("v")
    signatures: dict[str, dict[str, str]] = {}

    for platform, name in ARCHIVES.items():
        archive = dist / name
        if not archive.exists():
            print(f"skipping {name}, it is not part of this build", file=sys.stderr)
            continue
        sha256 = sha256_of(archive)
        url = download_url(slug, version, name)
        result = subprocess.run(
            [args.binary, args.key, url, sha256],
            capture_output=True,
            text=True,
            check=True,
        )
        signatures[name] = {
            "platform": platform,
            "url": url,
            "sha256": sha256,
            "signature": result.stdout.strip(),
        }

    (dist / "signatures.json").write_text(
        json.dumps(signatures, indent=2) + "\n", encoding="utf-8"
    )
    print(f"signed {len(signatures)} asset(s)")
    return 0


def cmd_manifest(args: argparse.Namespace) -> int:
    dist = pathlib.Path(args.dist)
    slug = repository_slug()
    version = args.version.lstrip("v")
    channel = args.channel or read_version_toml().get("channel", "stable")

    signatures: dict[str, dict[str, str]] = {}
    signature_file = dist / "signatures.json"
    if signature_file.exists():
        signatures = json.loads(signature_file.read_text(encoding="utf-8"))

    assets = []
    for platform, name in ARCHIVES.items():
        archive = dist / name
        if not archive.exists():
            continue
        sha256 = sha256_of(archive)
        signed = signatures.get(name, {})
        assets.append(
            {
                "platform": platform,
                "url": signed.get("url") or download_url(slug, version, name),
                "size": archive.stat().st_size,
                "sha256": sha256,
                "signature": signed.get("signature", ""),
            }
        )

    manifest = {
        "version": version,
        "channel": channel,
        "publishedAt": dt.datetime.now(dt.timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ"),
        "notesUrl": f"https://github.com/{slug}/releases/tag/v{version}",
        "minSupportedVersion": None,
        "assets": assets,
    }
    target = dist / "update-manifest.json"
    target.write_text(json.dumps(manifest, indent=2) + "\n", encoding="utf-8")
    print(f"wrote {target} with {len(assets)} asset(s)")
    return 0


# ---------------------------------------------------------------------------
# entry point
# ---------------------------------------------------------------------------


def main(argv: list[str]) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    sub = parser.add_subparsers(dest="command", required=True)

    resolve = sub.add_parser("resolve", help="print the release version")
    resolve.add_argument("--version", default="")
    resolve.set_defaults(func=cmd_resolve)

    bump = sub.add_parser("bump", help="write the next version into the repository")
    bump.add_argument(
        "--bump",
        default="patch",
        choices=["patch", "minor", "major", "none"],
    )
    bump.set_defaults(func=cmd_bump)

    sign = sub.add_parser("sign", help="sign the release assets")
    sign.add_argument("--key", required=True, help="Ed25519 private key in hex")
    sign.add_argument("--binary", required=True, help="path to the ferrox-sign binary")
    sign.add_argument("--version", required=True)
    sign.add_argument("--dist", default="dist")
    sign.set_defaults(func=cmd_sign)

    manifest = sub.add_parser("manifest", help="write the update manifest")
    manifest.add_argument("--version", required=True)
    manifest.add_argument("--channel", default="")
    manifest.add_argument("--dist", default="dist")
    manifest.set_defaults(func=cmd_manifest)

    args = parser.parse_args(argv)
    return args.func(args)


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
