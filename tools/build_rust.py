#!/usr/bin/env python3
# Copyright 2026 The Lynx Authors. All rights reserved.
# Licensed under the Apache License Version 2.0 that can be found in the
# LICENSE file in the root directory of this source tree.

"""Cross-platform build driver for the pure Rust AnimaX core & pure Rust Skity renderer.

Supports:
  - host    : native staticlib + cdylib (`libanimax_ffi.a` / `.so` / `.dylib`)
  - android : builds `libanimax_ffi.a` via Cargo and links `libanimax.so` with NDK clang++
              into `platform/android/animax_android/src/main/jniLibs/<abi>/libanimax.so`
  - ios     : aarch64-apple-ios, aarch64-apple-ios-sim (`libanimax_ffi.a` + optional XCFramework)
  - harmony : aarch64-unknown-linux-ohos
  - wasm    : wasm32-unknown-unknown (`animax_wasm.wasm` -> platform/web/core/public/build/)
"""

import argparse
import os
import platform
import shutil
import subprocess
import sys

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
RUST_DIR = os.path.join(ROOT, "rust")

ANDROID_TARGETS = {
    "arm64-v8a": ("aarch64-linux-android", "aarch64-linux-android"),
    "armeabi-v7a": ("armv7-linux-androideabi", "armv7a-linux-androideabi"),
    "x86_64": ("x86_64-linux-android", "x86_64-linux-android"),
    "x86": ("i686-linux-android", "i686-linux-android"),
}

IOS_TARGETS = [
    ("aarch64-apple-ios", "ios-arm64"),
    ("aarch64-apple-ios-sim", "ios-sim-arm64"),
]

HARMONY_TARGETS = [
    ("aarch64-unknown-linux-ohos", "arm64-v8a"),
]


def run(cmd, cwd=RUST_DIR, env=None):
    print("+", " ".join(cmd), flush=True)
    subprocess.check_call(cmd, cwd=cwd, env=env)


def find_android_ndk():
    candidates = [
        os.environ.get("ANDROID_NDK_HOME"),
        os.environ.get("ANDROID_NDK_ROOT"),
        os.environ.get("NDK_HOME"),
    ]
    sdk_roots = [
        os.environ.get("ANDROID_HOME"),
        os.environ.get("ANDROID_SDK_ROOT"),
        "/opt/android-sdk",
        os.path.expanduser("~/Android/Sdk"),
    ]
    for sdk in sdk_roots:
        if sdk and os.path.isdir(os.path.join(sdk, "ndk")):
            versions = sorted(os.listdir(os.path.join(sdk, "ndk")), reverse=True)
            for v in versions:
                candidates.append(os.path.join(sdk, "ndk", v))
    for c in candidates:
        if c and os.path.isdir(os.path.join(c, "toolchains", "llvm", "prebuilt")):
            return c
    raise RuntimeError(
        "Android NDK not found. Set ANDROID_NDK_HOME or install NDK under ANDROID_HOME/ndk/."
    )


def ndk_host_tag(ndk_dir):
    prebuilt = os.path.join(ndk_dir, "toolchains", "llvm", "prebuilt")
    sys_name = platform.system().lower()
    preferred = (
        ["darwin-x86_64", "darwin-arm64"]
        if sys_name == "darwin"
        else ["linux-x86_64"]
    )
    for tag in preferred:
        if os.path.isdir(os.path.join(prebuilt, tag)):
            return tag
    entries = os.listdir(prebuilt)
    if not entries:
        raise RuntimeError(f"Empty NDK prebuilt dir: {prebuilt}")
    return entries[0]


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


def build_android(release: bool, abis: list, api_level: int):
    ndk = find_android_ndk()
    host_tag = ndk_host_tag(ndk)
    bin_dir = os.path.join(ndk, "toolchains", "llvm", "prebuilt", host_tag, "bin")
    ar = os.path.join(bin_dir, "llvm-ar")
    profile = "release" if release else "debug"

    jni_libs_root = os.path.join(
        ROOT, "platform", "android", "animax_android", "src", "main", "jniLibs"
    )

    for abi in abis:
        if abi not in ANDROID_TARGETS:
            raise ValueError(f"Unsupported Android ABI: {abi}")
        rust_triple, clang_prefix = ANDROID_TARGETS[abi]
        clang = os.path.join(bin_dir, f"{clang_prefix}{api_level}-clang")
        clangxx = os.path.join(bin_dir, f"{clang_prefix}{api_level}-clang++")
        if not os.path.isfile(clangxx):
            raise RuntimeError(f"NDK compiler not found: {clangxx}")

        env = os.environ.copy()
        triple_env = rust_triple.upper().replace("-", "_")
        triple_cc = rust_triple.replace("-", "_")
        env[f"CARGO_TARGET_{triple_env}_LINKER"] = clang
        env[f"CARGO_TARGET_{triple_env}_AR"] = ar
        env[f"CC_{triple_cc}"] = clang
        env[f"CXX_{triple_cc}"] = clangxx
        env[f"AR_{triple_cc}"] = ar

        # Build staticlib only via cargo rustc or normal cargo build
        cargo_cmd = [
            "cargo",
            "build",
            "-p",
            "animax-ffi",
            "--lib",
            "--target",
            rust_triple,
        ]
        if release:
            cargo_cmd.append("--release")
        run(cargo_cmd, cwd=RUST_DIR, env=env)

        staticlib = os.path.join(
            RUST_DIR, "target", rust_triple, profile, "libanimax_ffi.a"
        )
        if not os.path.isfile(staticlib):
            raise RuntimeError(f"Missing Rust staticlib: {staticlib}")

        out_dir = os.path.join(jni_libs_root, abi)
        os.makedirs(out_dir, exist_ok=True)
        out_so = os.path.join(out_dir, "libanimax.so")

        link_cmd = [
            clangxx,
            "-shared",
            "-std=c++17",
            "-fPIC",
            "-O2" if release else "-O0",
            "-g" if not release else "-O2",
            f"-I{ROOT}",
            os.path.join(ROOT, "src", "rust_bridge", "animax_rust_bridge.cc"),
            os.path.join(ROOT, "src", "rust_bridge", "android_jni_bridge.cc"),
            staticlib,
            "-static-libstdc++",
            "-Wl,-z,max-page-size=16384",
            "-Wl,--gc-sections",
            "-landroid",
            "-ljnigraphics",
            "-llog",
            "-lm",
            "-ldl",
            "-o",
            out_so,
        ]
        run(link_cmd, cwd=ROOT, env=env)
        print(f"Built Android {abi}: {out_so} ({os.path.getsize(out_so)} bytes)")


def build_ios(release: bool, check_only: bool = False):
    profile = "release" if release else "debug"
    built_libs = []
    for triple, label in IOS_TARGETS:
        if check_only:
            cmd = ["cargo", "check", "-p", "animax-ffi", "--target", triple]
        else:
            cmd = [
                "cargo",
                "rustc",
                "-p",
                "animax-ffi",
                "--crate-type",
                "staticlib",
                "--target",
                triple,
            ]
        if release:
            cmd.append("--release")
        run(cmd, cwd=RUST_DIR)
        if not check_only:
            lib = os.path.join(RUST_DIR, "target", triple, profile, "libanimax_ffi.a")
            dst_dir = os.path.join(
                ROOT, "platform", "darwin", "ios", "rust_libs", label
            )
            os.makedirs(dst_dir, exist_ok=True)
            dst = os.path.join(dst_dir, "libanimax_ffi.a")
            shutil.copy2(lib, dst)
            built_libs.append((label, dst))
            print(f"Built iOS {label} ({triple}): {dst} ({os.path.getsize(dst)} bytes)")

    if not check_only and platform.system() == "Darwin" and len(built_libs) == 2:
        xcfw = os.path.join(
            ROOT, "platform", "darwin", "ios", "rust_libs", "AnimaXRustCore.xcframework"
        )
        if os.path.exists(xcfw):
            shutil.rmtree(xcfw)
        hdr_dir = os.path.join(ROOT, "include")
        cmd = ["xcodebuild", "-create-xcframework"]
        for _, lib_path in built_libs:
            cmd += ["-library", lib_path, "-headers", hdr_dir]
        cmd += ["-output", xcfw]
        run(cmd, cwd=ROOT)
        print(f"Created XCFramework: {xcfw}")


def build_harmony(release: bool):
    for triple, abi in HARMONY_TARGETS:
        cmd = [
            "cargo",
            "rustc",
            "-p",
            "animax-ffi",
            "--crate-type",
            "staticlib",
            "--target",
            triple,
        ]
        if release:
            cmd.append("--release")
        run(cmd, cwd=RUST_DIR)
        print(f"Built harmony/{abi} ({triple})")


def main():
    parser = argparse.ArgumentParser(description="Build AnimaX Rust core")
    parser.add_argument(
        "--target",
        choices=["host", "android", "ios", "harmony", "wasm"],
        default="host",
    )
    parser.add_argument("--debug", action="store_true")
    parser.add_argument(
        "--abis",
        default="arm64-v8a,armeabi-v7a,x86_64",
        help="Comma-separated Android ABIs to build",
    )
    parser.add_argument(
        "--api",
        type=int,
        default=21,
        help="Android NDK API level (default: 21)",
    )
    parser.add_argument(
        "--check-only",
        action="store_true",
        help="For iOS on non-macOS hosts, only run cargo check",
    )
    args = parser.parse_args()
    release = not args.debug

    if args.target == "host":
        build_host(release)
    elif args.target == "wasm":
        build_wasm(release)
    elif args.target == "android":
        abis = [a.strip() for a in args.abis.split(",") if a.strip()]
        build_android(release, abis, args.api)
    elif args.target == "ios":
        build_ios(release, check_only=args.check_only)
    else:
        build_harmony(release)


if __name__ == "__main__":
    main()
