#!/usr/bin/env bash
# Copyright 2026 The Lynx Authors. All rights reserved.
# Licensed under the Apache License Version 2.0 that can be found in the
# LICENSE file in the root directory of this source tree.

set -euo pipefail

# Build script for AnimaXExample iOS.
# Defaults to the pure Rust AnimaX core + Rust Skity renderer (USE_RUST_CORE=1).
# Set USE_RUST_CORE=0 to use the legacy C++/Skity/GN CocoaPods build.

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "${SCRIPT_DIR}/../../../.." && pwd)"
cd "${SCRIPT_DIR}"

: "${USE_RUST_CORE:=1}"
: "${CONFIGURATION:=Debug}"
: "${SCHEME:=AnimaXExample}"
: "${WORKSPACE:=AnimaXExample.xcworkspace}"
: "${ARCH:=arm64}"
: "${SDK:=iphonesimulator}"
: "${BUILD_DIR:=build}"

export DISABLE_CUSTOM_GEMS_CACHE=${DISABLE_CUSTOM_GEMS_CACHE:-0}

echo "[1/4] Cleaning previous artifacts"
rm -rf Pods Podfile.lock "${BUILD_DIR}" results_bundle* ../../../.seer/seer.yml ../../../.seer/information.yml

if [[ "${USE_RUST_CORE}" == "1" ]]; then
  echo "[2/4] Building pure Rust AnimaX + Rust Skity XCFramework"
  python3 "${ROOT_DIR}/tools/build_rust.py" --target ios
  python3 "${ROOT_DIR}/tools/ios_tools/generate_rust_podspec.py"

  echo "[3/4] Installing CocoaPods (pure Rust core)"
  export ANIMAX_USE_RUST_CORE=1
  pod install --verbose
else
  echo "[2/4] Xcode clean (${CONFIGURATION})"
  xcodebuild clean \
    -workspace "${WORKSPACE}" \
    -scheme "${SCHEME}" \
    -configuration "${CONFIGURATION}" \
    -quiet || true

  echo "[3/4] Installing bundles/pods (legacy C++ core)"
  export ANIMAX_USE_RUST_CORE=0
  ./bundle_install.sh
fi

echo "[4/4] Building (${SDK}/${ARCH}) → ${BUILD_DIR}"
mkdir -p "${BUILD_DIR}"

xcodebuild \
  -workspace "${WORKSPACE}" \
  -scheme "${SCHEME}" \
  -configuration "${CONFIGURATION}" \
  -arch "${ARCH}" \
  -sdk "${SDK}" \
  SYMROOT="$(pwd)/${BUILD_DIR}" \
  CODE_SIGN_IDENTITY="" \
  CODE_SIGNING_REQUIRED=NO
echo "Build completed. Products are in ${BUILD_DIR}/"
