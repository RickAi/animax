// Copyright 2026 The Lynx Authors. All rights reserved.
// Licensed under the Apache License Version 2.0 that can be found in the
// LICENSE file in the root directory of this source tree.

/**
 * Drop-in ES module loader replacing Emscripten `animax_wasm.js`.
 * Backed 100% by the pure Rust `animax-wasm` + `animax-skity` engine.
 */

const PIXEL_FORMAT_RGBA8888_STRAIGHT = 1;
const EVENT_NAMES = {
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

const encoder = new TextEncoder();
const decoder = new TextDecoder();

function withBytes(exports, bytes, fn) {
  const ptr = exports.animax_wasm_alloc(bytes.length);
  new Uint8Array(exports.memory.buffer, ptr, bytes.length).set(bytes);
  try {
    return fn(ptr, bytes.length);
  } finally {
    exports.animax_wasm_free(ptr, bytes.length);
  }
}

function withCString(exports, str, fn) {
  const raw = encoder.encode(str);
  const bytes = new Uint8Array(raw.length + 1);
  bytes.set(raw);
  bytes[raw.length] = 0;
  return withBytes(exports, bytes, (ptr) => fn(ptr, raw.length));
}

function readCString(exports, ptr) {
  if (!ptr) return '';
  const mem = new Uint8Array(exports.memory.buffer);
  let end = ptr;
  while (mem[end] !== 0) end++;
  return decoder.decode(mem.subarray(ptr, end));
}

function resolveCanvasBySelector(selector) {
  if (!selector || typeof document === 'undefined') return null;
  if (selector.startsWith('#')) {
    const id = selector.slice(1);
    const direct = document.getElementById(id);
    if (direct) return direct;
    // Also search shadow roots of <animax-view> elements
    for (const el of document.querySelectorAll('animax-view')) {
      const found = el.shadowRoot && el.shadowRoot.getElementById(id);
      if (found) return found;
    }
  }
  return document.querySelector(selector);
}

export default async function animax_wasm(moduleOverrides = {}) {
  const wasmUrl =
    typeof moduleOverrides.locateFile === 'function'
      ? moduleOverrides.locateFile('animax_wasm.wasm', '')
      : new URL('./animax_wasm.wasm', import.meta.url).href;

  const response = await fetch(wasmUrl);
  if (!response.ok) {
    throw new Error(`Failed to fetch animax_wasm.wasm from ${wasmUrl}: ${response.status}`);
  }
  const bytes = await response.arrayBuffer();
  const { instance } = await WebAssembly.instantiate(bytes, {});
  const exports = instance.exports;

  const registeredFonts = [];
  const activeInstances = new Set();

  class AnimaXWebGPUContext {
    static create() {
      return new AnimaXWebGPUContext();
    }
    delete() {}
  }

  class AnimaXPlayer {
    constructor(owner) {
      this._owner = owner;
      this._handle = owner._handle;
      this._polyfill = {};
      this._imageFolder = '';
    }
    setLoop(v) { exports.animax_player_set_loop(this._handle, v ? 1 : 0); }
    setLoopCount(v) { exports.animax_player_set_loop_count(this._handle, v | 0); }
    setProgress(v) {
      exports.animax_player_set_progress(this._handle, Number(v) || 0);
      this._owner._onDirty();
    }
    setAutoReverse(v) { exports.animax_player_set_auto_reverse(this._handle, v ? 1 : 0); }
    setAutoPlay(v) { exports.animax_player_set_autoplay(this._handle, v ? 1 : 0); }
    setSpeed(v) { exports.animax_player_set_speed(this._handle, Number(v) || 1); }
    setObjectFit(v) {
      exports.animax_player_set_object_fit(this._handle, v & 0xff);
      this._owner._onDirty();
    }
    setObjectPosition(v) {
      exports.animax_player_set_object_position(this._handle, v & 0xff);
      this._owner._onDirty();
    }
    setKeepLastFrame(v) { exports.animax_player_set_keep_last_frame(this._handle, v ? 1 : 0); }
    setJson(json) {
      const bytes = encoder.encode(String(json || ''));
      withBytes(exports, bytes, (ptr, len) =>
        exports.animax_player_set_json(this._handle, ptr, len),
      );
      this._owner._onDirty();
    }
    setSrc(url) {
      const resolved = this._polyfill[url] || url;
      if (!resolved) return;
      fetch(resolved)
        .then(async (res) => {
          if (!res.ok) throw new Error(`HTTP ${res.status}`);
          const buf = new Uint8Array(await res.arrayBuffer());
          // Detect ZIP magic PK\x03\x04 vs JSON text
          if (buf.length >= 4 && buf[0] === 0x50 && buf[1] === 0x4b && buf[2] === 0x03 && buf[3] === 0x04) {
            withBytes(exports, buf, (ptr, len) =>
              exports.animax_player_set_zip(this._handle, ptr, len),
            );
          } else {
            withBytes(exports, buf, (ptr, len) =>
              exports.animax_player_set_json(this._handle, ptr, len),
            );
          }
          this._owner._onDirty();
        })
        .catch((err) => {
          if (this._owner._eventCb) {
            this._owner._eventCb('error', { code: 1, msg: String(err) });
          }
        });
    }
    setImageFolder(folder) { this._imageFolder = String(folder || ''); }
    setSrcPolyfill(map) { this._polyfill = map || {}; }
    setStartFrame(f) { exports.animax_player_set_start_frame(this._handle, Number(f) || 0); }
    setEndFrame(f) {
      if (Number(f) >= 0) exports.animax_player_set_end_frame(this._handle, Number(f));
    }
    setFpsEventInterval(_ms) {}
    setMaxFrameRate(fps) { exports.animax_player_set_max_frame_rate(this._handle, Number(fps) || 60); }
    setDynamicResource(_v) {}
    play() {
      exports.animax_player_play(this._handle);
      this._owner._onDirty();
    }
    pause() { exports.animax_player_pause(this._handle); }
    resume() {
      exports.animax_player_resume(this._handle);
      this._owner._onDirty();
    }
    stop() {
      exports.animax_player_stop(this._handle);
      this._owner._onDirty();
    }
    reload() {
      exports.animax_player_reload(this._handle);
      this._owner._onDirty();
    }
    getDurationMs() { return Number(exports.animax_player_get_duration_ms(this._handle)); }
    isAnimating() { return exports.animax_player_is_animating(this._handle) !== 0; }
    seek(frame) {
      exports.animax_player_seek(this._handle, Number(frame) || 0);
      this._owner._onDirty();
    }
    getCurrentFrame() { return exports.animax_player_get_current_frame(this._handle); }
    playSegment(startFrame, endFrame) {
      exports.animax_player_play_segment(this._handle, Number(startFrame) || 0, Number(endFrame) || 0);
      this._owner._onDirty();
    }
    onShow(state) {
      exports.animax_player_on_show(this._handle, state & 0xff);
      this._owner._onDirty();
    }
    onHide(state) { exports.animax_player_on_hide(this._handle, state & 0xff); }
    onTap(x, y) {
      exports.animax_player_on_tap(this._handle, Number(x) || 0, Number(y) || 0);
      this._owner._flushEvents();
    }
    subscribeUpdateEvent(frame) { exports.animax_player_subscribe_update_event(this._handle, frame | 0); }
    unsubscribeUpdateEvent(frame) { exports.animax_player_unsubscribe_update_event(this._handle, frame | 0); }
    subscribeUpdateEvents(frames, sub) {
      for (const f of frames || []) {
        if (sub) this.subscribeUpdateEvent(f);
        else this.unsubscribeUpdateEvent(f);
      }
    }
    delete() {}
  }

  class AnimaXWasm {
    constructor(deviceScale = 1.0) {
      this._handle = exports.animax_player_create(Number(deviceScale) || 1.0);
      this._player = new AnimaXPlayer(this);
      this._ctx2d = null;
      this._width = 0;
      this._height = 0;
      this._rafId = 0;
      this._eventCb = null;
      this._captureCb = null;
      this._deleted = false;
      activeInstances.add(this);
      for (const f of registeredFonts) {
        this._registerFont(f.family, f.data);
      }
    }

    static onResourceLoadSuccess() {}
    static onResourceLoadError() {}
    static registerFontWithData(family, _isDefault, _priority, data) {
      const bytes = data instanceof Uint8Array ? data : new Uint8Array(data);
      registeredFonts.push({ family, data: bytes });
      for (const inst of activeInstances) {
        inst._registerFont(family, bytes);
      }
      return true;
    }

    _registerFont(family, bytes) {
      const nameBytes = encoder.encode(family);
      withBytes(exports, nameBytes, (np, nl) =>
        withBytes(exports, bytes, (dp, dl) =>
          exports.animax_player_register_font(this._handle, np, nl, dp, dl),
        ),
      );
    }

    getPlayer() { return this._player; }
    setMaxFrameRate(fps) { exports.animax_player_set_max_frame_rate(this._handle, Number(fps) || 60); }
    setResourceLoaderImpl(_loader) {}

    setSurfaceWithCanvas(canvasSelector, width, height, _webgpuCtx, _glHandle) {
      const canvas =
        typeof canvasSelector === 'string'
          ? resolveCanvasBySelector(canvasSelector)
          : canvasSelector;
      this._width = Math.max(1, width | 0);
      this._height = Math.max(1, height | 0);
      if (canvas) {
        canvas.width = this._width;
        canvas.height = this._height;
        this._ctx2d = canvas.getContext('2d');
      }
      exports.animax_player_set_surface_size(this._handle, this._width, this._height);
      this._onDirty();
    }

    updateVisibilityStates(flags) {
      exports.animax_player_update_visibility_states(this._handle, flags & 0xff);
      if ((flags & 0xff) === 0) this._ensureLoop();
    }

    setEventCallback(cb) {
      this._eventCb = cb;
      this._flushEvents();
    }

    setFrameCaptureCallback(cb) { this._captureCb = cb; }

    requestFrameCapture() {
      const pixels = this._readPixels();
      if (pixels && this._captureCb) {
        this._captureCb(pixels, this._width, this._height);
      }
    }

    setResourceProperty(_type, _id, _val, cb) {
      if (cb) cb(true, '');
    }

    updateLayerProperty(propType, keyPathStr, valueParam, cb) {
      const csv = Array.isArray(keyPathStr) ? keyPathStr.join(',') : String(keyPathStr || '');
      const targetFrame =
        valueParam && typeof valueParam.frameIndex === 'number' ? valueParam.frameIndex : -1;
      const val = valueParam && 'value' in valueParam ? valueParam.value : valueParam;
      let ok = false;

      if (typeof val === 'boolean') {
        ok = withCString(exports, csv, (kp) =>
          exports.animax_player_set_layer_bool(this._handle, kp, propType, val ? 1 : 0, targetFrame),
        ) !== 0;
      } else if (typeof val === 'number') {
        // Color properties: 103 (TextColor), 201 (Color), 203 (StrokeColor), 219 (DropShadowColor)
        if (propType === 103 || propType === 201 || propType === 203 || propType === 219) {
          ok = withCString(exports, csv, (kp) =>
            exports.animax_player_set_layer_color(this._handle, kp, propType, val | 0, targetFrame),
          ) !== 0;
        } else {
          ok = withCString(exports, csv, (kp) =>
            exports.animax_player_set_layer_number(this._handle, kp, propType, val, targetFrame),
          ) !== 0;
        }
      } else if (typeof val === 'string') {
        if (val.startsWith('#') && (propType === 103 || propType === 201 || propType === 203 || propType === 219)) {
          const argb = withCString(exports, val, (s) => exports.animax_parse_hex_color(s));
          ok = withCString(exports, csv, (kp) =>
            exports.animax_player_set_layer_color(this._handle, kp, propType, argb, targetFrame),
          ) !== 0;
        } else {
          ok = withCString(exports, csv, (kp) =>
            withCString(exports, val, (vp) =>
              exports.animax_player_set_layer_string(this._handle, kp, propType, vp, targetFrame),
            ),
          ) !== 0;
        }
      } else if (val && typeof val === 'object' && 'x' in val && 'y' in val) {
        ok = withCString(exports, csv, (kp) =>
          exports.animax_player_set_layer_coord(
            this._handle,
            kp,
            propType,
            Number(val.x) || 0,
            Number(val.y) || 0,
            Number(val.z) || 0,
            targetFrame,
          ),
        ) !== 0;
      }

      this._onDirty();
      if (cb) cb(ok, ok ? '' : 'unsupported property value');
    }

    getLayerBounds(layerName, boundsSpace, cb) {
      const bufPtr = exports.animax_wasm_alloc(16);
      try {
        const ok = withCString(exports, String(layerName || ''), (np) =>
          exports.animax_player_get_layer_bounds(
            this._handle,
            np,
            boundsSpace === 1 ? 1 : 0,
            bufPtr,
            bufPtr + 4,
            bufPtr + 8,
            bufPtr + 12,
          ),
        ) !== 0;
        if (ok) {
          const dv = new DataView(exports.memory.buffer, bufPtr, 16);
          const l = dv.getFloat32(0, true);
          const t = dv.getFloat32(4, true);
          const r = dv.getFloat32(8, true);
          const b = dv.getFloat32(12, true);
          if (cb) cb(true, l, t, r - l, b - t);
        } else if (cb) {
          cb(false, 0, 0, 0, 0);
        }
      } finally {
        exports.animax_wasm_free(bufPtr, 16);
      }
    }

    getKeysForKeyPath(keyPathStr, cb) {
      const csv = Array.isArray(keyPathStr) ? keyPathStr.join(',') : String(keyPathStr || '**');
      const ptr = withCString(exports, csv, (kp) =>
        exports.animax_player_get_keys_for_key_path_json(this._handle, kp),
      );
      if (!ptr) {
        if (cb) cb([]);
        return;
      }
      try {
        const parsed = JSON.parse(readCString(exports, ptr));
        if (cb) cb(parsed);
      } finally {
        exports.animax_string_free(ptr);
      }
    }

    delete() {
      if (this._deleted) return;
      this._deleted = true;
      activeInstances.delete(this);
      if (this._rafId) {
        cancelAnimationFrame(this._rafId);
        this._rafId = 0;
      }
      exports.animax_player_destroy(this._handle);
    }

    _onDirty() {
      if (this._deleted) return;
      this._blit();
      this._flushEvents();
      this._ensureLoop();
    }

    _ensureLoop() {
      if (this._deleted || this._rafId || exports.animax_player_is_animating(this._handle) === 0) {
        return;
      }
      const step = (nowMs) => {
        this._rafId = 0;
        if (this._deleted) return;
        const tsNs = BigInt(Math.round(nowMs * 1_000_000));
        if (exports.animax_player_tick(this._handle, tsNs) !== 0) {
          this._blit();
        }
        this._flushEvents();
        if (exports.animax_player_is_animating(this._handle) !== 0) {
          this._rafId = requestAnimationFrame(step);
        }
      };
      this._rafId = requestAnimationFrame(step);
    }

    _readPixels() {
      if (this._width <= 0 || this._height <= 0) return null;
      const lenPtr = exports.animax_wasm_alloc(8);
      try {
        const pxPtr = exports.animax_player_get_pixels(
          this._handle,
          PIXEL_FORMAT_RGBA8888_STRAIGHT,
          lenPtr,
        );
        if (!pxPtr) return null;
        const byteLen = new DataView(exports.memory.buffer).getUint32(lenPtr, true);
        return new Uint8ClampedArray(exports.memory.buffer.slice(pxPtr, pxPtr + byteLen));
      } finally {
        exports.animax_wasm_free(lenPtr, 8);
      }
    }

    _blit() {
      if (!this._ctx2d || this._width <= 0 || this._height <= 0) return;
      const pixels = this._readPixels();
      if (!pixels || pixels.length < this._width * this._height * 4) return;
      this._ctx2d.putImageData(new ImageData(pixels, this._width, this._height), 0, 0);
    }

    _flushEvents() {
      if (!this._eventCb) return;
      const ptr = exports.animax_player_drain_events_json(this._handle);
      if (!ptr) return;
      try {
        const list = JSON.parse(readCString(exports, ptr));
        for (const item of list) {
          const name = EVENT_NAMES[item.event];
          if (name) {
            this._eventCb(name, item.params || {});
          }
        }
      } finally {
        exports.animax_string_free(ptr);
      }
    }
  }

  return {
    isRustCore: true,
    AnimaXWasm,
    AnimaXPlayer,
    AnimaXWebGPUContext,
    GL: null,
    loadDynamicLibrary: async () => true,
  };
}
