#!/usr/bin/env bash
# Copyright 2026 The Lynx Authors. All rights reserved.
# Licensed under the Apache License Version 2.0 that can be found in the
# LICENSE file in the root directory of this source tree.

set -euo pipefail

# Minimal build script for AnimaXExample Android (compile-only).
# Defaults to the pure Rust AnimaX core + Rust Skity renderer (USE_RUST_CORE=1).
# Set USE_RUST_CORE=0 to use the legacy C++/GN/Skity pipeline.

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd -P)"
ROOT_DIR="$(cd "${SCRIPT_DIR}/../.." && pwd -P)"
ANDROID_DIR="${ROOT_DIR}/example/android"
APP_MODULE_DIR="${ANDROID_DIR}/animax_example"
BUILD_DIR="${APP_MODULE_DIR}/build"

USE_RUST_CORE="${USE_RUST_CORE:-1}"
ANDROID_ABIS="${ANDROID_ABIS:-arm64-v8a,armeabi-v7a,x86_64}"

echo "[1/3] Cleaning previous artifacts"
rm -rf "${BUILD_DIR}" || true

if [[ "${USE_RUST_CORE}" == "1" ]]; then
  echo "[2/3] Building pure Rust AnimaX + Rust Skity native libraries (${ANDROID_ABIS})"
  pushd "${ROOT_DIR}" >/dev/null
  python3 tools/build_rust.py --target android --abis "${ANDROID_ABIS}"
  popd >/dev/null

  echo "[3/3] Building APK (pure Rust core) → ${BUILD_DIR}"
  pushd "${ANDROID_DIR}" >/dev/null
  ./gradlew :AnimaXExample:assembleNoasanDebug -Pndk27=true -PuseRustCore=true -PabiList="${ANDROID_ABIS}"
  popd >/dev/null
else
  # Source environment (build tools, SDK/NDK, Python venv)
  source "${ROOT_DIR}/tools/envsetup.sh"

  echo "[2/3] Syncing external dependencies"
  pushd "${ROOT_DIR}" >/dev/null
  ${ROOT_DIR}/tools/hab sync .
  popd >/dev/null

  echo "[3/3] Building (legacy C++ core) → ${BUILD_DIR}"
  pushd "${ANDROID_DIR}" >/dev/null
  ./gradlew :AnimaXExample:assembleNoasanDebug -PuseRustCore=false
  popd >/dev/null
fi

# Locate the built APK and print its absolute path.
APK_CANDIDATE="${BUILD_DIR}/outputs/apk/noasan/debug/AnimaXExample-noasan-debug.apk"
if [[ ! -f "${APK_CANDIDATE}" ]]; then
  APK_CANDIDATE=$(find "${BUILD_DIR}/outputs/apk" -type f -name '*debug*.apk' | head -n 1 || true)
fi

if [[ -n "${APK_CANDIDATE}" && -f "${APK_CANDIDATE}" ]]; then
  APK_ABS_DIR="$(cd "$(dirname "${APK_CANDIDATE}")" && pwd -P)"
  APK_ABS_PATH="${APK_ABS_DIR}/$(basename "${APK_CANDIDATE}")"
  echo "Build completed. APK: ${APK_ABS_PATH}"
else
  echo "Build completed. Products are in ${BUILD_DIR}/ (debug APK not found)" >&2
fi
