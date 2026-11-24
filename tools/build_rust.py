#!/usr/bin/env python3
# Copyright 2026 The Lynx Authors. All rights reserved.
# Licensed under the Apache License Version 2.0 that can be found in the
# LICENSE file in the root directory of this source tree.

"""Cross-platform build driver for the pure Rust AnimaX core & pure Rust Skity renderer.

Supports:
  - host    : native staticlib + cdylib (`libanimax_ffi.a` / `.so` / `.dylib`)
  - android : aarch64-linux-android, armv7-linux-androideabi, x86_64-linux-android, i686-linux-android
  - ios     : aarch64-apple-ios, aarch64-apple-ios-sim
  - harmony : aarch64-unknown-linux-ohos
  - wasm    : wasm32-unknown-unknown (`animax_wasm.wasm` -> platform/web/core/public/build/)
"""

import argparse
import os
import shutil
import subprocess
import sys

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
RUST_DIR = os.path.join(ROOT, "rust")

TARGET_MAP = {
    "android": [
        ("aarch64-linux-android", "arm64-v8a"),
        ("armv7-linux-androideabi", "armeabi-v7a"),
        ("x86_64-linux-android", "x86_64"),
        ("i686-linux-android", "x86"),
    ],
    "ios": [
        ("aarch64-apple-ios", "ios-arm64"),
        ("aarch64-apple-ios-sim", "ios-sim-arm64"),
    ],
    "harmony": [
        ("aarch64-unknown-linux-ohos", "arm64-v8a"),
    ],
}


def run(cmd):
    print("+", " ".join(cmd), flush=True)
    subprocess.check_call(cmd, cwd=RUST_DIR)


def build_host(release: bool):
    cmd = ["cargo", "build", "-p", "animax-ffi"]
    if release:
        cmd.append("--release")
    run(cmd)


def build_wasm(release: bool):
    cmd = [
        "cargo",
        "build",
        "-p",
        "animax-wasm",
        "--target",
        "wasm32-unknown-unknown",
    ]
    if release:
        cmd.append("--release")
    run(cmd)
    profile = "release" if release else "debug"
    src = os.path.join(
        RUST_DIR, "target", "wasm32-unknown-unknown", profile, "animax_wasm.wasm"
    )
    dst_dir = os.path.join(ROOT, "platform", "web", "core", "public", "build")
    os.makedirs(dst_dir, exist_ok=True)
    dst = os.path.join(dst_dir, "animax_wasm.wasm")
    shutil.copy2(src, dst)
    print(f"Copied WASM artifact to {dst} ({os.path.getsize(dst)} bytes)")


def build_mobile(platform: str, release: bool):
    for triple, abi in TARGET_MAP[platform]:
        cmd = ["cargo", "build", "-p", "animax-ffi", "--target", triple]
        if release:
            cmd.append("--release")
        run(cmd)
        print(f"Built {platform}/{abi} ({triple})")


def main():
    parser = argparse.ArgumentParser(description="Build AnimaX Rust core")
    parser.add_argument(
        "--target",
        choices=["host", "android", "ios", "harmony", "wasm"],
        default="host",
    )
    parser.add_argument("--debug", action="store_true")
    args = parser.parse_args()
    release = not args.debug

    if args.target == "host":
        build_host(release)
    elif args.target == "wasm":
        build_wasm(release)
    else:
        build_mobile(args.target, release)


if __name__ == "__main__":
    main()
