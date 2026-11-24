// Copyright 2026 The Lynx Authors. All rights reserved.
// Licensed under the Apache License Version 2.0 that can be found in the
// LICENSE file in the root directory of this source tree.

#include "src/rust_bridge/animax_rust_bridge.h"

#include <chrono>
#include <cmath>
#include <cstdio>
#include <cstdlib>
#include <string>
#include <vector>

using lynx::animax::RustPlayerBridge;

namespace {

void Check(bool cond, const char* msg) {
  if (!cond) {
    std::fprintf(stderr, "FAIL: %s\n", msg);
    std::exit(1);
  }
}

const char* kRichLottieJson = R"JSON({
  "v": "5.7.4",
  "fr": 60,
  "ip": 0,
  "op": 120,
  "w": 256,
  "h": 256,
  "nm": "RustBenchComp",
  "layers": [
    {
      "ty": 4,
      "ind": 1,
      "nm": "AnimatedBadge",
      "ip": 0,
      "op": 120,
      "ks": {
        "o": {"a": 0, "k": 100},
        "r": {"a": 1, "k": [{"t": 0, "s": [0], "e": [360]}, {"t": 120, "s": [360]}]},
        "p": {"a": 1, "k": [{"t": 0, "s": [64, 128], "e": [192, 128]}, {"t": 120, "s": [192, 128]}]},
        "s": {"a": 0, "k": [100, 100]}
      },
      "shapes": [
        {
          "ty": "gr",
          "nm": "BadgeGroup",
          "it": [
            {"ty": "rc", "nm": "Card", "p": {"a": 0, "k": [0, 0]}, "s": {"a": 0, "k": [80, 80]}, "r": {"a": 0, "k": 12}},
            {"ty": "fl", "nm": "CardFill", "c": {"a": 0, "k": [1, 0, 0, 1]}, "o": {"a": 0, "k": 100}}
          ]
        },
        {
          "ty": "gr",
          "nm": "RingGroup",
          "it": [
            {"ty": "el", "nm": "Ring", "p": {"a": 0, "k": [0, 0]}, "s": {"a": 0, "k": [110, 110]}},
            {"ty": "st", "nm": "RingStroke", "c": {"a": 0, "k": [1, 1, 1, 1]}, "o": {"a": 0, "k": 100}, "w": {"a": 0, "k": 4}},
            {"ty": "tm", "nm": "Trim", "s": {"a": 0, "k": 0}, "e": {"a": 1, "k": [{"t": 0, "s": [25], "e": [100]}, {"t": 120, "s": [100]}]}, "o": {"a": 0, "k": 0}}
          ]
        }
      ]
    },
    {
      "ty": 1,
      "ind": 2,
      "nm": "BackgroundSolid",
      "sw": 256,
      "sh": 256,
      "sc": "#FF102030",
      "ip": 0,
      "op": 120,
      "ks": {"o": {"a": 0, "k": 100}, "p": {"a": 0, "k": [0, 0]}, "s": {"a": 0, "k": [100, 100]}}
    }
  ]
})JSON";

}  // namespace

int main() {
  // 1. Utility FFI parity
  Check(animax_parse_hex_color("#FF112233") == static_cast<int32_t>(0xFF112233),
        "animax_parse_hex_color");

  // 2. End-to-end player lifecycle via C++ -> Rust FFI
  RustPlayerBridge player(1.0f);
  Check(player.SetSurfaceSize(256, 256), "SetSurfaceSize");
  Check(player.SetJson(kRichLottieJson), "SetJson");
  Check(player.GetDurationMs() == 2000, "GetDurationMs == 2000");

  std::vector<uint8_t> rgba(256 * 256 * 4, 0);
  Check(player.CopyPixelsToBuffer(rgba.data(), 256 * 4, 256,
                                  ANIMAX_PIXEL_FORMAT_RGBA8888_PREMUL),
        "CopyPixelsToBuffer RGBA");

  // Background solid at (4, 4) should be RGB(0x10, 0x20, 0x30)
  const size_t bg_off = (4 * 256 + 4) * 4;
  Check(rgba[bg_off] == 0x10 && rgba[bg_off + 1] == 0x20 &&
            rgba[bg_off + 2] == 0x30 && rgba[bg_off + 3] == 0xFF,
        "Background solid pixel matches #FF102030");

  // Red card centered at (64, 128) at frame 0
  const size_t card_off = (128 * 256 + 64) * 4;
  Check(rgba[card_off] == 255 && rgba[card_off + 1] == 0 &&
            rgba[card_off + 2] == 0 && rgba[card_off + 3] == 255,
        "CardFill pixel is red at frame 0");

  // BGRA format for iOS Metal/CoreGraphics presentation
  std::vector<uint8_t> bgra(256 * 256 * 4, 0);
  Check(player.CopyPixelsToBuffer(bgra.data(), 256 * 4, 256,
                                  ANIMAX_PIXEL_FORMAT_BGRA8888_PREMUL),
        "CopyPixelsToBuffer BGRA");
  Check(bgra[card_off] == 0 && bgra[card_off + 1] == 0 &&
            bgra[card_off + 2] == 255 && bgra[card_off + 3] == 255,
        "BGRA byte swap verified");

  // Dynamic property override: recolor CardFill to pure green (property 201 = kColor)
  Check(player.SetLayerColor("AnimatedBadge,BadgeGroup,CardFill", 201,
                             static_cast<int32_t>(0xFF00FF00)),
        "SetLayerColor");
  Check(player.CopyPixelsToBuffer(rgba.data(), 256 * 4, 256,
                                  ANIMAX_PIXEL_FORMAT_RGBA8888_PREMUL),
        "CopyPixels after dynamic color");
  Check(rgba[card_off] == 0 && rgba[card_off + 1] == 255 &&
            rgba[card_off + 2] == 0 && rgba[card_off + 3] == 255,
        "CardFill dynamically recolored to green");

  // Hit-testing & event draining
  Check(player.OnTap(64.0f, 128.0f) >= 1, "OnTap hits AnimatedBadge");
  std::string events = player.DrainEventsJson();
  Check(events.find("AnimatedBadge") != std::string::npos,
        "DrainEventsJson contains TapLayer AnimatedBadge");

  // 3. Rendering throughput benchmark (120 frames @ 256x256 pure Rust Skity)
  player.SetLoop(true);
  player.Play();
  constexpr int kFrames = 120;
  auto t0 = std::chrono::steady_clock::now();
  for (int i = 0; i < kFrames; ++i) {
    player.Tick(static_cast<int64_t>(i) * 16'666'667LL);
  }
  auto t1 = std::chrono::steady_clock::now();
  double total_ms = std::chrono::duration<double, std::milli>(t1 - t0).count();
  double ms_per_frame = total_ms / kFrames;
  std::printf(
      "PASS: C++ <-> Rust FFI end-to-end verified! Rendered %d frames (256x256) "
      "in %.2f ms (%.3f ms/frame, %.1f FPS)\n",
      kFrames, total_ms, ms_per_frame, 1000.0 / ms_per_frame);
  return 0;
}
