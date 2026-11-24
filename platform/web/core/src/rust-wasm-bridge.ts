// Copyright 2026 The Lynx Authors. All rights reserved.
// Licensed under the Apache License Version 2.0 that can be found in the
// LICENSE file in the root directory of this source tree.

/**
 * Pure Rust WebAssembly bridge for `@lynx-js/animax` (`platform/web/core`).
 *
 * Replaces Emscripten + C++ Skity WASM with `animax_wasm.wasm` compiled from
 * `rust/animax-wasm`, implementing the exact `AnimaXWasm` + `AnimaXPlayer`
 * API consumed by `AnimaXViewElement` (`platform/web/core/src/view.ts`) and
 * blitting straight RGBA8888 frames directly onto `<canvas>` 2D `ImageData`.
 */

const PIXEL_FORMAT_RGBA8888_STRAIGHT = 1;

interface RustWasmExports {
  memory: WebAssembly.Memory;
  animax_wasm_alloc(len: number): number;
  animax_wasm_free(ptr: number, len: number): void;
  animax_player_create(deviceScale: number): number;
  animax_player_destroy(handle: number): void;
  animax_player_set_surface_size(handle: number, width: number, height: number): number;
  animax_player_get_surface_size(handle: number, outW: number, outH: number): void;
  animax_player_set_json(handle: number, ptr: number, len: number): number;
  animax_player_set_zip(handle: number, ptr: number, len: number): number;
  animax_player_reload(handle: number): void;
  animax_player_play(handle: number): void;
  animax_player_pause(handle: number): void;
  animax_player_resume(handle: number): void;
  animax_player_stop(handle: number): void;
  animax_player_seek(handle: number, frame: number): void;
  animax_player_set_progress(handle: number, progress: number): void;
  animax_player_play_segment(handle: number, startFrame: number, endFrame: number): void;
  animax_player_set_loop(handle: number, loop: number): void;
  animax_player_set_loop_count(handle: number, count: number): void;
  animax_player_set_auto_reverse(handle: number, autoReverse: number): void;
  animax_player_set_autoplay(handle: number, autoplay: number): void;
  animax_player_set_speed(handle: number, speed: number): void;
  animax_player_set_start_frame(handle: number, frame: number): void;
  animax_player_set_end_frame(handle: number, frame: number): void;
  animax_player_set_max_frame_rate(handle: number, fps: number): void;
  animax_player_set_keep_last_frame(handle: number, keep: number): void;
  animax_player_set_object_fit(handle: number, fit: number): void;
  animax_player_set_object_position(handle: number, pos: number): void;
  animax_player_get_duration_ms(handle: number): bigint;
  animax_player_is_animating(handle: number): number;
  animax_player_get_current_frame(handle: number): number;
  animax_player_on_show(handle: number, state: number): void;
  animax_player_on_hide(handle: number, state: number): void;
  animax_player_update_visibility_states(handle: number, flags: number): void;
  animax_player_on_tap(handle: number, x: number, y: number): number;
  animax_player_tick(handle: number, timestampNs: bigint): number;
  animax_player_render(handle: number): number;
  animax_player_get_pixels(handle: number, format: number, outLenPtr: number): number;
  animax_player_register_font(
    handle: number,
    namePtr: number,
    nameLen: number,
    ttfPtr: number,
    ttfLen: number,
  ): number;
  animax_player_register_image_encoded(
    handle: number,
    idPtr: number,
    idLen: number,
    dataPtr: number,
    dataLen: number,
  ): number;
  animax_player_subscribe_update_event(handle: number, frame: number): void;
  animax_player_unsubscribe_update_event(handle: number, frame: number): void;
  animax_player_drain_events_json(handle: number): number;
  animax_string_free(ptr: number): void;
}

const EVENT_NAMES: Record<number, string> = {
  0: 'completion',
  1: 'start',
  2: 'repeat',
  3: 'cancel',
  4: 'ready',
  5: 'update',
  6: 'error',
  7: 'fps',
  8: 'compositionReady',
  9: 'tapLayer',
  10: 'firstFrame',
  11: 'warning',
};

const textEncoder = new TextEncoder();
const textDecoder = new TextDecoder();

function withRustBytes<T>(
  exports: RustWasmExports,
  bytes: Uint8Array,
  fn: (ptr: number, len: number) => T,
): T {
  const ptr = exports.animax_wasm_alloc(bytes.length);
  new Uint8Array(exports.memory.buffer, ptr, bytes.length).set(bytes);
  try {
    return fn(ptr, bytes.length);
  } finally {
    exports.animax_wasm_free(ptr, bytes.length);
  }
}

function readCString(exports: RustWasmExports, ptr: number): string {
  if (!ptr) return '';
  const mem = new Uint8Array(exports.memory.buffer);
  let end = ptr;
  while (mem[end] !== 0) end++;
  return textDecoder.decode(mem.subarray(ptr, end));
}

export class RustAnimaXPlayerWrapper {
  constructor(
    private readonly exports: RustWasmExports,
    private readonly handle: number,
    private readonly onDirty: () => void,
  ) {}

  setLoop(v: boolean) {
    this.exports.animax_player_set_loop(this.handle, v ? 1 : 0);
  }
  setLoopCount(v: number) {
    this.exports.animax_player_set_loop_count(this.handle, v | 0);
  }
  setProgress(v: number) {
    this.exports.animax_player_set_progress(this.handle, v);
    this.onDirty();
  }
  setAutoReverse(v: boolean) {
    this.exports.animax_player_set_auto_reverse(this.handle, v ? 1 : 0);
  }
  setAutoPlay(v: boolean) {
    this.exports.animax_player_set_autoplay(this.handle, v ? 1 : 0);
  }
  setSpeed(v: number) {
    this.exports.animax_player_set_speed(this.handle, v);
  }
  setObjectFit(fit: number) {
    this.exports.animax_player_set_object_fit(this.handle, fit & 0xff);
    this.onDirty();
  }
  setObjectPosition(pos: number) {
    this.exports.animax_player_set_object_position(this.handle, pos & 0xff);
    this.onDirty();
  }
  setKeepLastFrame(v: boolean) {
    this.exports.animax_player_set_keep_last_frame(this.handle, v ? 1 : 0);
  }
  setJson(json: string) {
    const bytes = textEncoder.encode(json);
    withRustBytes(this.exports, bytes, (ptr, len) =>
      this.exports.animax_player_set_json(this.handle, ptr, len),
    );
    this.onDirty();
  }
  setZipBytes(zipBytes: Uint8Array) {
    withRustBytes(this.exports, zipBytes, (ptr, len) =>
      this.exports.animax_player_set_zip(this.handle, ptr, len),
    );
    this.onDirty();
  }
  setStartFrame(frame: number) {
    this.exports.animax_player_set_start_frame(this.handle, frame);
  }
  setEndFrame(frame: number) {
    this.exports.animax_player_set_end_frame(this.handle, frame);
  }
  setMaxFrameRate(fps: number) {
    this.exports.animax_player_set_max_frame_rate(this.handle, fps);
  }
  play() {
    this.exports.animax_player_play(this.handle);
    this.onDirty();
  }
  pause() {
    this.exports.animax_player_pause(this.handle);
  }
  resume() {
    this.exports.animax_player_resume(this.handle);
    this.onDirty();
  }
  stop() {
    this.exports.animax_player_stop(this.handle);
    this.onDirty();
  }
  reload() {
    this.exports.animax_player_reload(this.handle);
    this.onDirty();
  }
  seek(frame: number) {
    this.exports.animax_player_seek(this.handle, frame);
    this.onDirty();
  }
  playSegment(startFrame: number, endFrame: number) {
    this.exports.animax_player_play_segment(this.handle, startFrame, endFrame);
    this.onDirty();
  }
  getDurationMs(): number {
    return Number(this.exports.animax_player_get_duration_ms(this.handle));
  }
  isAnimating(): boolean {
    return this.exports.animax_player_is_animating(this.handle) !== 0;
  }
  getCurrentFrame(): number {
    return this.exports.animax_player_get_current_frame(this.handle);
  }
  onShow(state: number) {
    this.exports.animax_player_on_show(this.handle, state & 0xff);
    this.onDirty();
  }
  onHide(state: number) {
    this.exports.animax_player_on_hide(this.handle, state & 0xff);
  }
  onTap(x: number, y: number) {
    this.exports.animax_player_on_tap(this.handle, x, y);
    this.onDirty();
  }
  subscribeUpdateEvent(frame: number) {
    this.exports.animax_player_subscribe_update_event(this.handle, frame | 0);
  }
  unsubscribeUpdateEvent(frame: number) {
    this.exports.animax_player_unsubscribe_update_event(this.handle, frame | 0);
  }
  subscribeUpdateEvents(frames: number[], subscribe: boolean) {
    for (const f of frames) {
      if (subscribe) this.subscribeUpdateEvent(f);
      else this.unsubscribeUpdateEvent(f);
    }
  }
}

export class RustAnimaXWasmBridge {
  private readonly handle: number;
  private readonly player: RustAnimaXPlayerWrapper;
  private ctx2d: CanvasRenderingContext2D | null = null;
  private width = 0;
  private height = 0;
  private rafId = 0;
  private eventCallback: ((name: string, params: Record<string, unknown>) => void) | null = null;
  private frameCaptureCallback: ((pixels: Uint8ClampedArray, w: number, h: number) => void) | null =
    null;

  constructor(private readonly exports: RustWasmExports, deviceScale: number) {
    this.handle = exports.animax_player_create(deviceScale);
    this.player = new RustAnimaXPlayerWrapper(exports, this.handle, () => {
      this.blitFrame();
      this.flushEvents();
      this.ensureLoop();
    });
  }

  getPlayer(): RustAnimaXPlayerWrapper {
    return this.player;
  }

  setSurfaceWithCanvas(canvas: HTMLCanvasElement, width: number, height: number) {
    this.width = Math.max(1, width | 0);
    this.height = Math.max(1, height | 0);
    canvas.width = this.width;
    canvas.height = this.height;
    this.ctx2d = canvas.getContext('2d');
    this.exports.animax_player_set_surface_size(this.handle, this.width, this.height);
    this.blitFrame();
    this.flushEvents();
    this.ensureLoop();
  }

  updateVisibilityStates(flags: number) {
    this.exports.animax_player_update_visibility_states(this.handle, flags & 0xff);
  }

  setEventCallback(cb: (name: string, params: Record<string, unknown>) => void) {
    this.eventCallback = cb;
  }

  setFrameCaptureCallback(cb: (pixels: Uint8ClampedArray, w: number, h: number) => void) {
    this.frameCaptureCallback = cb;
  }

  registerFontWithData(name: string, ttfBytes: Uint8Array): boolean {
    const nameBytes = textEncoder.encode(name);
    return withRustBytes(this.exports, nameBytes, (np, nl) =>
      withRustBytes(
        this.exports,
        ttfBytes,
        (dp, dl) => this.exports.animax_player_register_font(this.handle, np, nl, dp, dl) !== 0,
      ),
    );
  }

  requestFrameCapture() {
    const pixels = this.readPixelsCopy();
    if (pixels && this.frameCaptureCallback) {
      this.frameCaptureCallback(pixels, this.width, this.height);
    }
  }

  delete() {
    if (this.rafId) {
      cancelAnimationFrame(this.rafId);
      this.rafId = 0;
    }
    this.exports.animax_player_destroy(this.handle);
  }

  private ensureLoop() {
    if (this.rafId || !this.player.isAnimating()) return;
    const step = (nowMs: number) => {
      this.rafId = 0;
      const tsNs = BigInt(Math.round(nowMs * 1_000_000));
      if (this.exports.animax_player_tick(this.handle, tsNs) !== 0) {
        this.blitFrame();
      }
      this.flushEvents();
      if (this.player.isAnimating()) {
        this.rafId = requestAnimationFrame(step);
      }
    };
    this.rafId = requestAnimationFrame(step);
  }

  private readPixelsCopy(): Uint8ClampedArray<ArrayBuffer> | null {
    if (this.width <= 0 || this.height <= 0) return null;
    const lenPtr = this.exports.animax_wasm_alloc(8);
    try {
      const ptr = this.exports.animax_player_get_pixels(
        this.handle,
        PIXEL_FORMAT_RGBA8888_STRAIGHT,
        lenPtr,
      );
      if (!ptr) return null;
      const byteLen = new DataView(this.exports.memory.buffer).getUint32(lenPtr, true);
      const out = new Uint8ClampedArray(new ArrayBuffer(byteLen));
      out.set(new Uint8Array(this.exports.memory.buffer, ptr, byteLen));
      return out;
    } finally {
      this.exports.animax_wasm_free(lenPtr, 8);
    }
  }

  private blitFrame() {
    if (!this.ctx2d) return;
    const pixels = this.readPixelsCopy();
    if (!pixels || pixels.length < this.width * this.height * 4) return;
    this.ctx2d.putImageData(new ImageData(pixels, this.width, this.height), 0, 0);
  }

  private flushEvents() {
    if (!this.eventCallback) return;
    const ptr = this.exports.animax_player_drain_events_json(this.handle);
    if (!ptr) return;
    try {
      const raw = readCString(this.exports, ptr);
      const list = JSON.parse(raw) as Array<{ event: number; params: Record<string, unknown> }>;
      for (const item of list) {
        const name = EVENT_NAMES[item.event];
        if (name) {
          this.eventCallback(name, item.params || {});
        }
      }
    } finally {
      this.exports.animax_string_free(ptr);
    }
  }
}

export async function loadAnimaXRustWasm(wasmUrl: string | URL | ArrayBuffer) {
  let instance: WebAssembly.Instance;
  if (wasmUrl instanceof ArrayBuffer) {
    const res = await WebAssembly.instantiate(wasmUrl, {});
    instance = res.instance;
  } else {
    const response = await fetch(wasmUrl);
    const bytes = await response.arrayBuffer();
    const res = await WebAssembly.instantiate(bytes, {});
    instance = res.instance;
  }
  const exports = instance.exports as unknown as RustWasmExports;
  return {
    createBridge(deviceScale = 1.0) {
      return new RustAnimaXWasmBridge(exports, deviceScale);
    },
  };
}
