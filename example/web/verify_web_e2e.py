#!/usr/bin/env python3
# Copyright 2026 The Lynx Authors. All rights reserved.
# Licensed under the Apache License Version 2.0 that can be found in the
# LICENSE file in the root directory of this source tree.

"""Live Headless Chromium E2E test verifying example/web/index.html (<animax-view>)
plays real Lottie animations on the pure Rust WASM + pure Rust Skity backend."""

import http.server
import os
import socketserver
import threading
from playwright.sync_api import sync_playwright

ROOT = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))


class QuietHandler(http.server.SimpleHTTPRequestHandler):
    def __init__(self, *args, **kwargs):
        super().__init__(*args, directory=ROOT, **kwargs)

    def log_message(self, format, *args):
        pass


def main():
    httpd = socketserver.TCPServer(("127.0.0.1", 0), QuietHandler)
    port = httpd.server_address[1]
    t = threading.Thread(target=httpd.serve_forever, daemon=True)
    t.start()

    errors = []
    with sync_playwright() as p:
        browser = p.chromium.launch()
        page = browser.new_page(viewport={"width": 640, "height": 640})
        page.on("pageerror", lambda e: errors.append(f"pageerror: {e}"))
        page.on(
            "console",
            lambda m: errors.append(f"console.{m.type}: {m.text}")
            if m.type == "error"
            else None,
        )

        page.goto(
            f"http://127.0.0.1:{port}/example/web/index.html",
            wait_until="networkidle",
        )
        page.wait_for_function(
            "() => window.__animaxView && typeof window.__animaxView.getDuration === 'function' && window.__animaxView.getDuration() > 0",
            timeout=15000,
        )

        def read_canvas_sample():
            return page.evaluate(
                """() => {
                const view = window.__animaxView;
                const canvas = view.querySelector('canvas') || (view.shadowRoot && view.shadowRoot.querySelector('canvas'));
                if (!canvas) return null;
                const ctx = canvas.getContext('2d');
                const img = ctx.getImageData(0, 0, canvas.width, canvas.height).data;
                let nonZero = 0;
                const sample = [];
                for (let i = 0; i < img.length; i += 64) {
                    if (img[i+3] > 0) nonZero++;
                    sample.push(img[i], img[i+1], img[i+2], img[i+3]);
                }
                return {
                    w: canvas.width,
                    h: canvas.height,
                    nonZero,
                    frame: view.getCurrentFrame(),
                    animating: view.isAnimating(),
                    duration: view.getDuration(),
                    sample
                };
            }"""
            )

        page.wait_for_timeout(150)
        s1 = read_canvas_sample()
        assert s1 is not None and s1["nonZero"] > 50 and s1["animating"]

        page.wait_for_timeout(450)
        s2 = read_canvas_sample()
        diff = sum(1 for a, b in zip(s1["sample"], s2["sample"]) if a != b)
        print(
            f"Web E2E simple_shape.json: duration={s1['duration']}ms "
            f"frame={s1['frame']:.2f}->{s2['frame']:.2f} nonZero={s1['nonZero']}/{s2['nonZero']} diff={diff}"
        )
        assert s2["frame"] != s1["frame"] and diff > 20

        page.click("#btnPause")
        page.wait_for_timeout(100)
        before = read_canvas_sample()
        page.click("#btnRecolor")
        page.wait_for_timeout(100)
        after = read_canvas_sample()
        recolor_diff = sum(1 for a, b in zip(before["sample"], after["sample"]) if a != b)
        print(f"Web E2E dynamic recolor diff={recolor_diff}")
        assert recolor_diff > 20

        page.select_option(
            "#sampleSelect",
            "../../example/android/animax_example/src/main/assets/sample/dp.json",
        )
        page.wait_for_timeout(400)
        sdp = read_canvas_sample()
        print(f"Web E2E dp.json: duration={sdp['duration']}ms nonZero={sdp['nonZero']}")
        assert sdp["nonZero"] > 50

        browser.close()

    httpd.shutdown()
    assert not errors, f"Browser console/page errors: {errors}"
    print("PASS: Web AnimaX Example verified live in Chromium!")


if __name__ == "__main__":
    main()
