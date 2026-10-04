"""Build and package a Pumpkin component with its exact API revision."""

import argparse
import hashlib
import json
from pathlib import Path
import re
import shutil
import subprocess
import tomllib

ROOT = Path(__file__).resolve().parents[1]


def run(*args):
    return subprocess.check_output(args, cwd=ROOT, text=True).strip()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--pumpkin-rev", help="Exact 40-character server Git commit")
    args = parser.parse_args()
    manifest = ROOT / "Cargo.toml"
    if args.pumpkin_rev:
        if not re.fullmatch(r"[0-9a-fA-F]{40}", args.pumpkin_rev):
            parser.error("--pumpkin-rev must be a full Git commit SHA")
        text = manifest.read_text(encoding="utf-8")
        text, count = re.subn(
            r'(pumpkin-plugin-api = \{[^\n]*rev = ")[0-9a-f]+(")',
            lambda m: m[1] + args.pumpkin_rev.lower() + m[2],
            text,
        )
        if count != 1:
            raise RuntimeError("Could not locate the Pumpkin API revision")
        manifest.write_text(text, encoding="utf-8")
        run("cargo", "update", "-p", "pumpkin-plugin-api")

    package = tomllib.loads(manifest.read_text(encoding="utf-8"))
    revision = package["dependencies"]["pumpkin-plugin-api"]["rev"]
    run("cargo", "build", "--release", "--locked", "--target", "wasm32-wasip2")
    component = ROOT / "target/wasm32-wasip2/release/chunky_pumpkin.wasm"
    # WASIp2 must be a component, not a core Wasm module.
    if component.read_bytes()[:8] != b"\0asm\x0d\0\x01\0":
        raise RuntimeError("Build output is not a WebAssembly component")

    dist = ROOT / "dist"
    dist.mkdir(exist_ok=True)
    shutil.copyfile(component, dist / "chunky.wasm")
    metadata = {
        "plugin_version": package["package"]["version"],
        "plugin_commit": run("git", "rev-parse", "HEAD"),
        "pumpkin_commit": revision,
        "pumpkin_source": "https://github.com/Pumpkin-MC/Pumpkin",
        "rustc": run("rustc", "--version"),
        "target": "wasm32-wasip2",
        "cargo_lock_sha256": hashlib.sha256((ROOT / "Cargo.lock").read_bytes()).hexdigest(),
        "wasm_sha256": hashlib.sha256(component.read_bytes()).hexdigest(),
    }
    (dist / "build-info.json").write_text(json.dumps(metadata, indent=2) + "\n", encoding="utf-8")
    (dist / "Cargo.lock").write_bytes((ROOT / "Cargo.lock").read_bytes())
    checksums = "".join(
        f"{hashlib.sha256((dist / name).read_bytes()).hexdigest()}  {name}\n"
        for name in ("chunky.wasm", "build-info.json", "Cargo.lock")
    )
    (dist / "SHA256SUMS").write_text(checksums, encoding="utf-8")
    print(json.dumps(metadata, indent=2))


if __name__ == "__main__":
    main()
