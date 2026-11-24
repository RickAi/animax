# AnimaX Rust Core & Pure Rust Skity Renderer

This workspace replaces the AnimaX C++ core (`src/`) and the external C++ Skity 2D graphics engine with a self-contained, zero-C++ pure Rust implementation, while keeping Android, iOS, HarmonyOS, and Web platform layers working through thin FFI/WASM bridges.

## Crate Architecture

| Crate | Replaces | Responsibility |
|---|---|---|
| `animax-base` | `src/base/util/`, `include/player/animax_fit_position.h`, `include/player/animax_event.h`, `include/base/visibility_state.h` | Color & hex parsing, math lerp/floor_mod/gamma_evaluate, `ObjectFit`/`ObjectPosition`, `VisibilityState`, `Event` enums, URI helpers |
| `animax-skity` | `third_party/skity` + `src/render/` | Pure Rust Skity-compatible 2D engine embedded in AnimaX: `Matrix` (4×4/3×3), `Quaternion`, `Camera`, `Path`, `PathBuilder`, `PathMeasure` (exact de Casteljau splitting), `Paint`, `Shader` (Linear/Radial), `PathEffect` (Dash), `MaskFilter` (Blur), `ColorFilter`, `Pixmap`, `AlphaVideoCompositor` (RGBA & YUV420p split), `FontManager`/`Font` (`fontdue`), `Canvas` (layers, masks, blend modes, luma mattes), `Surface` (RGBA8888Premul / RGBA8888Straight / BGRA8888Premul) |
| `animax-model` | `src/model/` | Lottie & AlphaPlayer data models (`CompositionModel`, `LayerModel`, 13 `ContentModel`s, `GradientColor` with O(N1+N2) mismatched stop merge, `ShapeData`, `DocumentData`, `Keyframe<T>`, `Animatable*`, assets, markers) |
| `animax-property` | `src/property/` | `AnimaXKeyPath` (`*`, `**`, `__container`), `LayerPropertyType`/`ResourcePropertyType`, `AnimaXValueParam`, `PropertyStore` |
| `animax-parser` | `src/parser/` + RapidJSON + zlib/unzip | Pure Rust Lottie JSON parser, AlphaPlayer `config.json` parser, `.zip` bundle unpacker |
| `animax-animation` | `src/animation/` + `src/animator/` | Cubic Bezier `PathInterpolator`, keyframe evaluators (including `PathMeasure` spatial motion paths & split dimensions), 2D/3D `TransformEvaluator`, `ValueAnimator` VSync state machine |
| `animax-engine` | `src/content/` + `src/layer/` + `src/player/` | Shape/modifier renderer, full layer tree (masks, alpha/luma mattes, blend modes, blur/drop-shadow, precomp, text, image, alpha video), hit-testing, `AnimaXPlayer` |
| `animax-ffi` | C++ core staticlib/cdylib | Zero-copy C ABI (`include/animax_rust_ffi.h`) + C++ RAII bridge (`src/rust_bridge/`) + Android/iOS/HarmonyOS surface adapters |
| `animax-wasm` | Emscripten + Skity WASM | Pure Rust `wasm32-unknown-unknown` module + TypeScript bridge (`platform/web/core/src/rust-wasm-bridge.ts`) |

## Build & Test

```bash
# Run all Rust unit & end-to-end rendering tests
cargo test --manifest-path rust/Cargo.toml

# Build host staticlib & cdylib (libanimax_ffi.a / .so)
python3 tools/build_rust.py --target host

# Compile & run C++ <-> Rust FFI integration test & frame benchmark
clang++ -O2 -std=c++17 -I. \
  src/rust_bridge/rust_ffi_test_bench.cc \
  src/rust_bridge/animax_rust_bridge.cc \
  rust/target/release/libanimax_ffi.a \
  -lpthread -ldl -lm -o /tmp/rust_ffi_test_bench && /tmp/rust_ffi_test_bench

# Build pure Rust WebAssembly for platform/web/core
python3 tools/build_rust.py --target wasm

# Cross-compile for mobile platforms (requires corresponding rustup target & NDK/SDK)
python3 tools/build_rust.py --target android
python3 tools/build_rust.py --target ios
python3 tools/build_rust.py --target harmony
```
