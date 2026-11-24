// Copyright 2026 The Lynx Authors. All rights reserved.
// Licensed under the Apache License Version 2.0 that can be found in the
// LICENSE file in the root directory of this source tree.

/**
 * Pure Rust-backed Android JNI bridge (`libanimax.so`).
 *
 * Implements every native JNI entry point declared by
 * `platform/android/animax_android` directly on top of `libanimax_ffi.a`
 * (pure Rust core + pure Rust `animax-skity` renderer) with zero dependency on
 * C++ Skity, RapidJSON, Lynx Base, or Textra.
 */

#include <android/bitmap.h>
#include <android/log.h>
#include <android/native_window.h>
#include <android/native_window_jni.h>
#include <jni.h>

#include <algorithm>
#include <atomic>
#include <cstdint>
#include <cstring>
#include <fstream>
#include <functional>
#include <memory>
#include <mutex>
#include <string>
#include <unordered_map>
#include <vector>

#include "include/animax_rust_ffi.h"
#include "src/rust_bridge/animax_rust_bridge.h"

namespace {

constexpr const char* kLogTag = "AnimaXRust";

JavaVM* g_jvm = nullptr;

JNIEnv* GetJNIEnv(bool* did_attach) {
  *did_attach = false;
  if (!g_jvm) return nullptr;
  JNIEnv* env = nullptr;
  jint res = g_jvm->GetEnv(reinterpret_cast<void**>(&env), JNI_VERSION_1_6);
  if (res == JNI_EDETACHED) {
    if (g_jvm->AttachCurrentThread(&env, nullptr) == JNI_OK) {
      *did_attach = true;
      return env;
    }
    return nullptr;
  }
  return res == JNI_OK ? env : nullptr;
}

void DetachIfNeeded(bool did_attach) {
  if (did_attach && g_jvm) {
    g_jvm->DetachCurrentThread();
  }
}

std::string JStringToStdString(JNIEnv* env, jstring jstr) {
  if (!jstr) return "";
  const char* chars = env->GetStringUTFChars(jstr, nullptr);
  if (!chars) return "";
  std::string out(chars);
  env->ReleaseStringUTFChars(jstr, chars);
  return out;
}

std::string ReadFileToString(const std::string& path) {
  std::ifstream ifs(path, std::ios::binary);
  if (!ifs) return "";
  return std::string((std::istreambuf_iterator<char>(ifs)),
                     std::istreambuf_iterator<char>());
}

struct LoaderHolder {
  std::mutex mu;
  // scheme ordinal -> GlobalRef(com.lynx.animax.loader.AnimaXLoader)
  std::unordered_map<int, jobject> loaders;

  void Clear(JNIEnv* env) {
    std::lock_guard<std::mutex> lock(mu);
    for (auto& kv : loaders) {
      if (kv.second) env->DeleteGlobalRef(kv.second);
    }
    loaders.clear();
  }
};

using LoaderCallback = std::function<void(JNIEnv*, const uint8_t*, size_t,
                                          const std::string& file_path,
                                          const std::string& error)>;

struct PendingLoaderCallback {
  LoaderCallback fn;
};

struct CompositionHandle {
  std::string json;
  std::vector<uint8_t> zip_bytes;
  bool is_zip = false;
  int64_t start_frame = 0;
  int64_t end_frame = 0;
};

struct AndroidRustPlayer {
  std::mutex mu;
  lynx::animax::RustPlayerBridge player;
  LoaderHolder* loader_holder = nullptr;
  jobject ability_ref = nullptr;  // GlobalRef(BaseAbility)
  jobject drawable_ref = nullptr; // GlobalRef(AnimaXSurfaceDrawable)
  ANativeWindow* native_window = nullptr;
  jobject bitmap_group_ref = nullptr; // GlobalRef(BitmapBufferGroup)
  std::unordered_map<std::string, std::string> src_polyfill;
  bool destroyed = false;

  explicit AndroidRustPlayer(float density) : player(density) {}

  void ReleaseSurfaceLocked(JNIEnv* env) {
    if (native_window) {
      ANativeWindow_release(native_window);
      native_window = nullptr;
    }
    if (bitmap_group_ref) {
      env->DeleteGlobalRef(bitmap_group_ref);
      bitmap_group_ref = nullptr;
    }
    if (drawable_ref) {
      env->DeleteGlobalRef(drawable_ref);
      drawable_ref = nullptr;
    }
  }

  void Destroy(JNIEnv* env) {
    std::lock_guard<std::mutex> lock(mu);
    destroyed = true;
    player.Stop();
    ReleaseSurfaceLocked(env);
    if (ability_ref) {
      env->DeleteGlobalRef(ability_ref);
      ability_ref = nullptr;
    }
  }

  bool PresentLocked(JNIEnv* env) {
    if (destroyed) return false;
    if (native_window) {
      ANativeWindow_Buffer buf{};
      if (ANativeWindow_lock(native_window, &buf, nullptr) == 0) {
        bool ok = player.CopyPixelsToBuffer(
            static_cast<uint8_t*>(buf.bits),
            static_cast<uint32_t>(buf.stride) * 4,
            static_cast<uint32_t>(buf.height),
            ANIMAX_PIXEL_FORMAT_RGBA8888_PREMUL);
        ANativeWindow_unlockAndPost(native_window);
        return ok;
      }
      return false;
    }
    if (bitmap_group_ref) {
      jclass group_cls = env->GetObjectClass(bitmap_group_ref);
      jmethodID get_buf_mid = env->GetMethodID(
          group_cls, "getBitmapBufferForOffscreenRendering",
          "()Lcom/lynx/animax/drawable/BitmapBuffer;");
      jmethodID on_update_mid =
          env->GetMethodID(group_cls, "onBufferUpdate", "()V");
      jobject buf_obj = env->CallObjectMethod(bitmap_group_ref, get_buf_mid);
      if (buf_obj) {
        jclass buf_cls = env->GetObjectClass(buf_obj);
        jmethodID get_bmp_mid =
            env->GetMethodID(buf_cls, "getBitmap", "()Landroid/graphics/Bitmap;");
        jmethodID set_state_mid = env->GetMethodID(buf_cls, "setState", "(I)V");
        jobject bmp = env->CallObjectMethod(buf_obj, get_bmp_mid);
        if (bmp) {
          env->CallVoidMethod(buf_obj, set_state_mid, 1 /* Prepare */);
          AndroidBitmapInfo info{};
          void* pixels = nullptr;
          if (AndroidBitmap_getInfo(env, bmp, &info) == ANDROID_BITMAP_RESULT_SUCCESS &&
              AndroidBitmap_lockPixels(env, bmp, &pixels) == ANDROID_BITMAP_RESULT_SUCCESS) {
            player.CopyPixelsToBuffer(static_cast<uint8_t*>(pixels), info.stride,
                                      info.height,
                                      ANIMAX_PIXEL_FORMAT_RGBA8888_PREMUL);
            AndroidBitmap_unlockPixels(env, bmp);
            env->CallVoidMethod(buf_obj, set_state_mid, 2 /* Ready */);
            env->CallVoidMethod(bitmap_group_ref, on_update_mid);
          }
          env->DeleteLocalRef(bmp);
        }
        env->DeleteLocalRef(buf_cls);
        env->DeleteLocalRef(buf_obj);
      }
      env->DeleteLocalRef(group_cls);
      return true;
    }
    return false;
  }

  void DispatchEventsLocked(JNIEnv* env) {
    if (!ability_ref || destroyed) return;
    std::string events_json = player.DrainEventsJson();
    if (events_json.size() <= 2) return;  // "[]"

    jclass ability_cls = env->GetObjectClass(ability_ref);
    jmethodID send_event_mid = env->GetMethodID(
        ability_cls, "sendEvent",
        "(ILcom/lynx/animax/base/bridge/ReadableMap;)V");
    jclass map_cls = env->FindClass("com/lynx/animax/base/bridge/JavaOnlyMap");
    jmethodID map_ctor = env->GetMethodID(map_cls, "<init>", "()V");
    jmethodID put_double =
        env->GetMethodID(map_cls, "putDouble", "(Ljava/lang/String;D)V");
    jstring k_current = env->NewStringUTF("current");
    jstring k_total = env->NewStringUTF("total");
    double cur_frame = static_cast<double>(player.GetCurrentFrame());
    double total_frame = static_cast<double>(
        animax_player_get_end_frame(player.raw()) -
        animax_player_get_start_frame(player.raw()));

    // Lightweight parser for `[{"event":N,...},...]`
    size_t pos = 0;
    while ((pos = events_json.find("\"event\":", pos)) != std::string::npos) {
      pos += 8;
      int event_id = std::atoi(events_json.c_str() + pos);
      jobject jmap = env->NewObject(map_cls, map_ctor);
      env->CallVoidMethod(jmap, put_double, k_current, cur_frame);
      env->CallVoidMethod(jmap, put_double, k_total, total_frame);
      env->CallVoidMethod(ability_ref, send_event_mid, event_id, jmap);
      env->DeleteLocalRef(jmap);
    }
    env->DeleteLocalRef(k_total);
    env->DeleteLocalRef(k_current);
    env->DeleteLocalRef(map_cls);
    env->DeleteLocalRef(ability_cls);
  }
};

// Global registry of active players for Choreographer VSync ticks
std::mutex g_players_mu;
std::vector<std::shared_ptr<AndroidRustPlayer>> g_active_players;
std::atomic<bool> g_vsync_requested{false};

void RequestAndroidVSync(JNIEnv* env) {
  bool expected = false;
  if (!g_vsync_requested.compare_exchange_strong(expected, true)) {
    return;
  }
  jclass vsync_cls = env->FindClass("com/lynx/animax/VSyncMonitor");
  if (vsync_cls) {
    jmethodID req_mid = env->GetStaticMethodID(vsync_cls, "requestVSync", "()V");
    if (req_mid) {
      env->CallStaticVoidMethod(vsync_cls, req_mid);
    }
    env->DeleteLocalRef(vsync_cls);
  }
}

void RegisterActivePlayer(const std::shared_ptr<AndroidRustPlayer>& p) {
  std::lock_guard<std::mutex> lock(g_players_mu);
  if (std::find(g_active_players.begin(), g_active_players.end(), p) ==
      g_active_players.end()) {
    g_active_players.push_back(p);
  }
}

void UnregisterActivePlayer(const std::shared_ptr<AndroidRustPlayer>& p) {
  std::lock_guard<std::mutex> lock(g_players_mu);
  g_active_players.erase(
      std::remove(g_active_players.begin(), g_active_players.end(), p),
      g_active_players.end());
}

std::shared_ptr<AndroidRustPlayer> GetPlayer(jlong ptr) {
  if (ptr == 0) return nullptr;
  auto* holder = reinterpret_cast<std::shared_ptr<AndroidRustPlayer>*>(ptr);
  return *holder;
}

int DetectSchemeOrdinal(const std::string& uri) {
  // AnimaXLoaderScheme: 0=ASSET, 1=FILE, 2=HTTP, 3=DATA_URL, 4=CONTENT_URI
  if (uri.rfind("http://", 0) == 0 || uri.rfind("https://", 0) == 0) return 2;
  if (uri.rfind("file://", 0) == 0) return 1;
  if (uri.rfind("data:", 0) == 0) return 3;
  if (uri.rfind("content://", 0) == 0) return 4;
  return 0;  // ASSET (default for asset:// or bare relative sample/... paths)
}

void DispatchUriLoad(JNIEnv* env, LoaderHolder* holder, const std::string& uri,
                     LoaderCallback cb) {
  if (!holder) {
    cb(env, nullptr, 0, "", "No loader holder registered");
    return;
  }
  int scheme = DetectSchemeOrdinal(uri);
  jobject loader_obj = nullptr;
  {
    std::lock_guard<std::mutex> lock(holder->mu);
    auto it = holder->loaders.find(scheme);
    if (it != holder->loaders.end()) {
      loader_obj = env->NewLocalRef(it->second);
    }
  }
  if (!loader_obj) {
    cb(env, nullptr, 0, "", "No loader for scheme");
    return;
  }

  std::string normalized = uri;
  if (scheme == 0 && uri.rfind("asset://", 0) != 0 && uri.rfind("file://", 0) != 0) {
    normalized = "asset://" + uri;
  }

  jclass req_cls = env->FindClass("com/lynx/animax/loader/AnimaXLoaderRequest");
  jmethodID create_req_mid = env->GetStaticMethodID(
      req_cls, "createRequest",
      "(Ljava/lang/String;)Lcom/lynx/animax/loader/AnimaXLoaderRequest;");
  jstring jurl = env->NewStringUTF(normalized.c_str());
  jobject req_obj = env->CallStaticObjectMethod(req_cls, create_req_mid, jurl);

  auto* pending = new PendingLoaderCallback{std::move(cb)};
  jclass ch_cls =
      env->FindClass("com/lynx/animax/loader/AnimaXLoaderCompletionHandler");
  jmethodID create_ch_mid = env->GetStaticMethodID(
      ch_cls, "createCompletionHandler",
      "(J)Lcom/lynx/animax/loader/AnimaXLoaderCompletionHandler;");
  jobject ch_obj = env->CallStaticObjectMethod(
      ch_cls, create_ch_mid, reinterpret_cast<jlong>(pending));

  jclass loader_cls = env->GetObjectClass(loader_obj);
  jmethodID load_mid = env->GetMethodID(
      loader_cls, "load",
      "(Lcom/lynx/animax/loader/AnimaXLoaderRequest;"
      "Lcom/lynx/animax/loader/AnimaXLoaderCompletionHandler;)V");
  env->CallVoidMethod(loader_obj, load_mid, req_obj, ch_obj);

  env->DeleteLocalRef(loader_cls);
  env->DeleteLocalRef(ch_obj);
  env->DeleteLocalRef(ch_cls);
  env->DeleteLocalRef(req_obj);
  env->DeleteLocalRef(jurl);
  env->DeleteLocalRef(req_cls);
  env->DeleteLocalRef(loader_obj);
}

std::string KeyPathToCsv(JNIEnv* env, jobject key_path_obj) {
  if (!key_path_obj) return "**";
  jclass kp_cls = env->GetObjectClass(key_path_obj);
  jmethodID get_keys_mid = env->GetMethodID(
      kp_cls, "getKeys", "()Lcom/lynx/animax/base/bridge/JavaOnlyArray;");
  jobject arr = env->CallObjectMethod(key_path_obj, get_keys_mid);
  std::string csv;
  if (arr) {
    jclass arr_cls = env->GetObjectClass(arr);
    jmethodID size_mid = env->GetMethodID(arr_cls, "size", "()I");
    jmethodID get_str_mid =
        env->GetMethodID(arr_cls, "getString", "(I)Ljava/lang/String;");
    jint n = env->CallIntMethod(arr, size_mid);
    for (jint i = 0; i < n; ++i) {
      auto jstr = static_cast<jstring>(env->CallObjectMethod(arr, get_str_mid, i));
      if (i > 0) csv += ",";
      csv += JStringToStdString(env, jstr);
      if (jstr) env->DeleteLocalRef(jstr);
    }
    env->DeleteLocalRef(arr_cls);
    env->DeleteLocalRef(arr);
  }
  env->DeleteLocalRef(kp_cls);
  return csv.empty() ? "**" : csv;
}

void ApplySurfaceDrawable(JNIEnv* env, AndroidRustPlayer& p,
                          jobject surface_drawable) {
  if (!surface_drawable) return;
  std::lock_guard<std::mutex> lock(p.mu);
  p.ReleaseSurfaceLocked(env);

  jclass cls = env->GetObjectClass(surface_drawable);
  jmethodID get_w = env->GetMethodID(cls, "getWidth", "()I");
  jmethodID get_h = env->GetMethodID(cls, "getHeight", "()I");
  jmethodID get_surface =
      env->GetMethodID(cls, "getSurface", "()Landroid/view/Surface;");
  jmethodID get_bmg = env->GetMethodID(
      cls, "getBitmapBufferGroup",
      "()Lcom/lynx/animax/drawable/BitmapBufferGroup;");
  jmethodID set_backend = env->GetMethodID(cls, "setActualBackend", "(I)V");

  int w = std::max(1, env->CallIntMethod(surface_drawable, get_w));
  int h = std::max(1, env->CallIntMethod(surface_drawable, get_h));

  jobject surface = env->CallObjectMethod(surface_drawable, get_surface);
  if (surface) {
    p.native_window = ANativeWindow_fromSurface(env, surface);
    if (p.native_window) {
      ANativeWindow_setBuffersGeometry(p.native_window, w, h,
                                       WINDOW_FORMAT_RGBA_8888);
    }
    env->DeleteLocalRef(surface);
  } else {
    jobject bmg = env->CallObjectMethod(surface_drawable, get_bmg);
    if (bmg) {
      p.bitmap_group_ref = env->NewGlobalRef(bmg);
      env->DeleteLocalRef(bmg);
    }
  }
  p.drawable_ref = env->NewGlobalRef(surface_drawable);
  // Report Software/RustSkity backend (Backend.SOFTWARE = 3)
  if (set_backend) {
    env->CallVoidMethod(surface_drawable, set_backend, 3);
  }
  env->DeleteLocalRef(cls);

  p.player.SetSurfaceSize(static_cast<uint32_t>(w), static_cast<uint32_t>(h));
  p.PresentLocked(env);
  p.DispatchEventsLocked(env);
}

// Composition factory scope -> loader holder
std::mutex g_factory_mu;
std::unordered_map<std::string, LoaderHolder*> g_factory_loaders;

}  // namespace

extern "C" {

JNIEXPORT jint JNICALL JNI_OnLoad(JavaVM* vm, void*) {
  g_jvm = vm;
  __android_log_print(ANDROID_LOG_INFO, kLogTag,
                      "Loaded pure Rust AnimaX + Rust Skity native library");
  return JNI_VERSION_1_6;
}

// ---- com.lynx.animax.util.AnimaX & AnimaXLog ----
JNIEXPORT void JNICALL
Java_com_lynx_animax_util_AnimaX_nativeRegisterLogger(JNIEnv*, jobject, jlong) {}

JNIEXPORT void JNICALL Java_com_lynx_animax_util_AnimaXLog_nativeLog(
    JNIEnv* env, jclass, jint level, jstring tag, jstring msg) {
  auto t = JStringToStdString(env, tag);
  auto m = JStringToStdString(env, msg);
  int prio = ANDROID_LOG_INFO;
  if (level >= 4) prio = ANDROID_LOG_ERROR;
  else if (level == 3) prio = ANDROID_LOG_WARN;
  __android_log_print(prio, t.empty() ? kLogTag : t.c_str(), "%s", m.c_str());
}

// ---- com.lynx.animax.VSyncMonitor ----
JNIEXPORT void JNICALL
Java_com_lynx_animax_VSyncMonitor_nativeInvokeCallback(JNIEnv* env, jclass,
                                                       jlong frame_time_nanos) {
  g_vsync_requested.store(false);
  std::vector<std::shared_ptr<AndroidRustPlayer>> snapshot;
  {
    std::lock_guard<std::mutex> lock(g_players_mu);
    snapshot = g_active_players;
  }
  bool any_animating = false;
  for (auto& p : snapshot) {
    std::lock_guard<std::mutex> lock(p->mu);
    if (p->destroyed) continue;
    if (p->player.Tick(static_cast<int64_t>(frame_time_nanos))) {
      p->PresentLocked(env);
    }
    p->DispatchEventsLocked(env);
    if (p->player.IsAnimating()) {
      any_animating = true;
    }
  }
  if (any_animating) {
    RequestAndroidVSync(env);
  }
}

// ---- com.lynx.animax.loader.AnimaXResourceLoaderHolder ----
JNIEXPORT jlong JNICALL
Java_com_lynx_animax_loader_AnimaXResourceLoaderHolder_nativeCreate(JNIEnv*,
                                                                    jclass) {
  return reinterpret_cast<jlong>(new LoaderHolder());
}

JNIEXPORT void JNICALL
Java_com_lynx_animax_loader_AnimaXResourceLoaderHolder_nativeDestroy(
    JNIEnv* env, jclass, jlong ptr) {
  if (!ptr) return;
  auto* holder = reinterpret_cast<LoaderHolder*>(ptr);
  holder->Clear(env);
  delete holder;
}

JNIEXPORT void JNICALL
Java_com_lynx_animax_loader_AnimaXResourceLoaderHolder_nativeRegisterLoaderForScheme(
    JNIEnv* env, jclass, jlong ptr, jobject loader, jint scheme) {
  if (!ptr || !loader) return;
  auto* holder = reinterpret_cast<LoaderHolder*>(ptr);
  std::lock_guard<std::mutex> lock(holder->mu);
  if (holder->loaders[scheme]) {
    env->DeleteGlobalRef(holder->loaders[scheme]);
  }
  holder->loaders[scheme] = env->NewGlobalRef(loader);
}

// ---- com.lynx.animax.loader.AnimaXLoaderCompletionHandler ----
JNIEXPORT void JNICALL
Java_com_lynx_animax_loader_AnimaXLoaderCompletionHandler_nativeOnComplete(
    JNIEnv* env, jobject, jlong native_ptr, jobject response) {
  if (!native_ptr) return;
  std::unique_ptr<PendingLoaderCallback> pending(
      reinterpret_cast<PendingLoaderCallback*>(native_ptr));
  if (!response) {
    pending->fn(env, nullptr, 0, "", "Null loader response");
    return;
  }
  jclass cls = env->GetObjectClass(response);
  jmethodID get_type = env->GetMethodID(cls, "getType", "()I");
  jint type = env->CallIntMethod(response, get_type);
  // AnimaXLoaderResponse.Type: 0=STRING_FILE_PATH, 1=BYTE_ARRAY, 2=BITMAP, 3=ERROR
  if (type == 3) {
    jmethodID get_err =
        env->GetMethodID(cls, "getErrorMessage", "()Ljava/lang/String;");
    auto jerr = static_cast<jstring>(env->CallObjectMethod(response, get_err));
    pending->fn(env, nullptr, 0, "", JStringToStdString(env, jerr));
    if (jerr) env->DeleteLocalRef(jerr);
  } else if (type == 0) {
    jmethodID get_fp =
        env->GetMethodID(cls, "getFilePath", "()Ljava/lang/String;");
    auto jfp = static_cast<jstring>(env->CallObjectMethod(response, get_fp));
    std::string fp = JStringToStdString(env, jfp);
    if (jfp) env->DeleteLocalRef(jfp);
    pending->fn(env, nullptr, 0, fp, "");
  } else if (type == 1) {
    jmethodID get_data = env->GetMethodID(cls, "getData", "()[B");
    auto jbytes =
        static_cast<jbyteArray>(env->CallObjectMethod(response, get_data));
    if (jbytes) {
      jsize len = env->GetArrayLength(jbytes);
      jbyte* ptr = env->GetByteArrayElements(jbytes, nullptr);
      pending->fn(env, reinterpret_cast<const uint8_t*>(ptr),
                  static_cast<size_t>(len), "", "");
      env->ReleaseByteArrayElements(jbytes, ptr, JNI_ABORT);
      env->DeleteLocalRef(jbytes);
    } else {
      pending->fn(env, nullptr, 0, "", "Empty raw data");
    }
  } else {
    pending->fn(env, nullptr, 0, "", "");
  }
  env->DeleteLocalRef(cls);
}

// ---- com.lynx.animax.AnimaXPlayer ----
JNIEXPORT jlong JNICALL Java_com_lynx_animax_AnimaXPlayer_nativeCreate(
    JNIEnv* env, jobject, jlong loader_ptr, jobject context) {
  float density = 1.0f;
  jobject ability_global = nullptr;
  if (context) {
    jclass ctx_cls = env->GetObjectClass(context);
    jmethodID get_density = env->GetMethodID(ctx_cls, "getDeviceDensity", "()F");
    if (get_density) density = env->CallFloatMethod(context, get_density);
    jmethodID get_ability = env->GetMethodID(
        ctx_cls, "getAbility", "()Lcom/lynx/animax/ability/BaseAbility;");
    if (get_ability) {
      jobject ab = env->CallObjectMethod(context, get_ability);
      if (ab) {
        ability_global = env->NewGlobalRef(ab);
        env->DeleteLocalRef(ab);
      }
    }
    env->DeleteLocalRef(ctx_cls);
  }

  auto player = std::make_shared<AndroidRustPlayer>(density);
  player->loader_holder = reinterpret_cast<LoaderHolder*>(loader_ptr);
  player->ability_ref = ability_global;
  RegisterActivePlayer(player);
  return reinterpret_cast<jlong>(new std::shared_ptr<AndroidRustPlayer>(player));
}

JNIEXPORT void JNICALL
Java_com_lynx_animax_AnimaXPlayer_nativeCreateAnimaXSurface(
    JNIEnv* env, jobject, jlong ptr, jobject surface_drawable) {
  if (auto p = GetPlayer(ptr)) {
    ApplySurfaceDrawable(env, *p, surface_drawable);
    if (p->player.IsAnimating()) RequestAndroidVSync(env);
  }
}

JNIEXPORT void JNICALL
Java_com_lynx_animax_AnimaXPlayer_nativeUpdateAnimaXSurface(
    JNIEnv* env, jobject, jlong ptr, jobject surface_drawable) {
  if (auto p = GetPlayer(ptr)) {
    ApplySurfaceDrawable(env, *p, surface_drawable);
  }
}

JNIEXPORT void JNICALL Java_com_lynx_animax_AnimaXPlayer_nativeSetAutoPlay(
    JNIEnv*, jobject, jlong ptr, jboolean enable) {
  if (auto p = GetPlayer(ptr)) {
    std::lock_guard<std::mutex> lock(p->mu);
    p->player.SetAutoplay(enable == JNI_TRUE);
  }
}

JNIEXPORT void JNICALL Java_com_lynx_animax_AnimaXPlayer_nativeSetSpeed(
    JNIEnv*, jobject, jlong ptr, jfloat speed) {
  if (auto p = GetPlayer(ptr)) {
    std::lock_guard<std::mutex> lock(p->mu);
    p->player.SetSpeed(speed);
  }
}

JNIEXPORT void JNICALL Java_com_lynx_animax_AnimaXPlayer_nativeSetProgress(
    JNIEnv* env, jobject, jlong ptr, jfloat progress) {
  if (auto p = GetPlayer(ptr)) {
    std::lock_guard<std::mutex> lock(p->mu);
    p->player.SetProgress(progress);
    p->PresentLocked(env);
  }
}

JNIEXPORT void JNICALL Java_com_lynx_animax_AnimaXPlayer_nativeSetObjectFit(
    JNIEnv* env, jobject, jlong ptr, jint fit) {
  if (auto p = GetPlayer(ptr)) {
    std::lock_guard<std::mutex> lock(p->mu);
    p->player.SetObjectFit(static_cast<uint8_t>(fit));
    p->player.Render();
    p->PresentLocked(env);
  }
}

JNIEXPORT void JNICALL
Java_com_lynx_animax_AnimaXPlayer_nativeSetObjectPosition(JNIEnv* env, jobject,
                                                          jlong ptr, jint pos) {
  if (auto p = GetPlayer(ptr)) {
    std::lock_guard<std::mutex> lock(p->mu);
    p->player.SetObjectPosition(static_cast<uint8_t>(pos));
    p->player.Render();
    p->PresentLocked(env);
  }
}

JNIEXPORT void JNICALL Java_com_lynx_animax_AnimaXPlayer_nativeSetKeepLastFrame(
    JNIEnv*, jobject, jlong ptr, jboolean enable) {
  if (auto p = GetPlayer(ptr)) {
    std::lock_guard<std::mutex> lock(p->mu);
    p->player.SetKeepLastFrame(enable == JNI_TRUE);
  }
}

JNIEXPORT void JNICALL Java_com_lynx_animax_AnimaXPlayer_nativeSetJson(
    JNIEnv* env, jobject, jlong ptr, jstring json) {
  if (auto p = GetPlayer(ptr)) {
    std::string s = JStringToStdString(env, json);
    bool animating = false;
    {
      std::lock_guard<std::mutex> lock(p->mu);
      p->player.SetJson(s);
      p->PresentLocked(env);
      p->DispatchEventsLocked(env);
      animating = p->player.IsAnimating();
    }
    if (animating) RequestAndroidVSync(env);
  }
}

JNIEXPORT void JNICALL Java_com_lynx_animax_AnimaXPlayer_nativeSetSrc(
    JNIEnv* env, jobject, jlong ptr, jstring src) {
  auto p = GetPlayer(ptr);
  if (!p) return;
  std::string uri = JStringToStdString(env, src);
  LoaderHolder* holder = nullptr;
  {
    std::lock_guard<std::mutex> lock(p->mu);
    auto it = p->src_polyfill.find(uri);
    if (it != p->src_polyfill.end()) uri = it->second;
    holder = p->loader_holder;
  }
  std::weak_ptr<AndroidRustPlayer> weak_p = p;
  DispatchUriLoad(
      env, holder, uri,
      [weak_p](JNIEnv* cb_env, const uint8_t* data, size_t len,
               const std::string& file_path, const std::string& err) {
        auto strong = weak_p.lock();
        if (!strong) return;
        bool animating = false;
        {
          std::lock_guard<std::mutex> lock(strong->mu);
          if (strong->destroyed) return;
          if (!err.empty()) {
            __android_log_print(ANDROID_LOG_ERROR, kLogTag,
                                "SetSrc load failed: %s", err.c_str());
            return;
          }
          std::string file_data;
          const uint8_t* bytes = data;
          size_t byte_len = len;
          if ((!bytes || byte_len == 0) && !file_path.empty()) {
            file_data = ReadFileToString(file_path);
            bytes = reinterpret_cast<const uint8_t*>(file_data.data());
            byte_len = file_data.size();
          }
          if (bytes && byte_len >= 4 && bytes[0] == 0x50 && bytes[1] == 0x4B &&
              bytes[2] == 0x03 && bytes[3] == 0x04) {
            strong->player.SetZip(bytes, byte_len);
          } else if (bytes && byte_len > 0) {
            strong->player.SetJson(
                std::string(reinterpret_cast<const char*>(bytes), byte_len));
          }
          strong->PresentLocked(cb_env);
          strong->DispatchEventsLocked(cb_env);
          animating = strong->player.IsAnimating();
        }
        if (animating) RequestAndroidVSync(cb_env);
      });
}

JNIEXPORT void JNICALL Java_com_lynx_animax_AnimaXPlayer_nativeSetComposition(
    JNIEnv* env, jobject, jlong ptr, jlong comp_ptr) {
  auto p = GetPlayer(ptr);
  auto* comp = reinterpret_cast<CompositionHandle*>(comp_ptr);
  if (!p || !comp) return;
  bool animating = false;
  {
    std::lock_guard<std::mutex> lock(p->mu);
    if (comp->is_zip) {
      p->player.SetZip(comp->zip_bytes.data(), comp->zip_bytes.size());
    } else {
      p->player.SetJson(comp->json);
    }
    p->PresentLocked(env);
    p->DispatchEventsLocked(env);
    animating = p->player.IsAnimating();
  }
  if (animating) RequestAndroidVSync(env);
}

JNIEXPORT void JNICALL Java_com_lynx_animax_AnimaXPlayer_nativeSetLoop(
    JNIEnv*, jobject, jlong ptr, jboolean enable) {
  if (auto p = GetPlayer(ptr)) {
    std::lock_guard<std::mutex> lock(p->mu);
    p->player.SetLoop(enable == JNI_TRUE);
  }
}

JNIEXPORT void JNICALL Java_com_lynx_animax_AnimaXPlayer_nativeSetLoopCount(
    JNIEnv*, jobject, jlong ptr, jint count) {
  if (auto p = GetPlayer(ptr)) {
    std::lock_guard<std::mutex> lock(p->mu);
    p->player.SetLoopCount(count);
  }
}

JNIEXPORT void JNICALL Java_com_lynx_animax_AnimaXPlayer_nativeSetImageFolder(
    JNIEnv*, jobject, jlong, jstring) {}

JNIEXPORT void JNICALL Java_com_lynx_animax_AnimaXPlayer_nativeSetSrcPolyfill(
    JNIEnv* env, jobject, jlong ptr, jobject polyfill_map) {
  auto p = GetPlayer(ptr);
  if (!p || !polyfill_map) return;
  jclass map_cls = env->GetObjectClass(polyfill_map);
  jmethodID get_keys =
      env->GetMethodID(map_cls, "getKeys", "()Ljava/util/ArrayList;");
  jmethodID get_str = env->GetMethodID(
      map_cls, "getString", "(Ljava/lang/String;)Ljava/lang/String;");
  jobject keys_list = env->CallObjectMethod(polyfill_map, get_keys);
  if (keys_list) {
    jclass list_cls = env->GetObjectClass(keys_list);
    jmethodID size_mid = env->GetMethodID(list_cls, "size", "()I");
    jmethodID get_mid =
        env->GetMethodID(list_cls, "get", "(I)Ljava/lang/Object;");
    jint n = env->CallIntMethod(keys_list, size_mid);
    std::lock_guard<std::mutex> lock(p->mu);
    p->src_polyfill.clear();
    for (jint i = 0; i < n; ++i) {
      auto jkey = static_cast<jstring>(env->CallObjectMethod(keys_list, get_mid, i));
      if (jkey) {
        auto jval =
            static_cast<jstring>(env->CallObjectMethod(polyfill_map, get_str, jkey));
        if (jval) {
          p->src_polyfill[JStringToStdString(env, jkey)] =
              JStringToStdString(env, jval);
          env->DeleteLocalRef(jval);
        }
        env->DeleteLocalRef(jkey);
      }
    }
    env->DeleteLocalRef(list_cls);
    env->DeleteLocalRef(keys_list);
  }
  env->DeleteLocalRef(map_cls);
}

JNIEXPORT void JNICALL Java_com_lynx_animax_AnimaXPlayer_nativeSetStartFrame(
    JNIEnv*, jobject, jlong ptr, jint frame) {
  if (auto p = GetPlayer(ptr)) {
    std::lock_guard<std::mutex> lock(p->mu);
    p->player.SetStartFrame(static_cast<float>(frame));
  }
}

JNIEXPORT void JNICALL Java_com_lynx_animax_AnimaXPlayer_nativeSetEndFrame(
    JNIEnv*, jobject, jlong ptr, jint frame) {
  if (auto p = GetPlayer(ptr)) {
    std::lock_guard<std::mutex> lock(p->mu);
    if (frame >= 0) p->player.SetEndFrame(static_cast<float>(frame));
  }
}

JNIEXPORT void JNICALL Java_com_lynx_animax_AnimaXPlayer_nativeSetAutoReverse(
    JNIEnv*, jobject, jlong ptr, jboolean enable) {
  if (auto p = GetPlayer(ptr)) {
    std::lock_guard<std::mutex> lock(p->mu);
    p->player.SetAutoReverse(enable == JNI_TRUE);
  }
}

JNIEXPORT void JNICALL Java_com_lynx_animax_AnimaXPlayer_nativePlay(JNIEnv* env,
                                                                    jobject,
                                                                    jlong ptr) {
  if (auto p = GetPlayer(ptr)) {
    {
      std::lock_guard<std::mutex> lock(p->mu);
      p->player.Play();
    }
    RequestAndroidVSync(env);
  }
}

JNIEXPORT void JNICALL Java_com_lynx_animax_AnimaXPlayer_nativePause(JNIEnv*,
                                                                     jobject,
                                                                     jlong ptr) {
  if (auto p = GetPlayer(ptr)) {
    std::lock_guard<std::mutex> lock(p->mu);
    p->player.Pause();
  }
}

JNIEXPORT void JNICALL Java_com_lynx_animax_AnimaXPlayer_nativeResume(
    JNIEnv* env, jobject, jlong ptr) {
  if (auto p = GetPlayer(ptr)) {
    {
      std::lock_guard<std::mutex> lock(p->mu);
      p->player.Resume();
    }
    RequestAndroidVSync(env);
  }
}

JNIEXPORT void JNICALL Java_com_lynx_animax_AnimaXPlayer_nativeStop(JNIEnv* env,
                                                                    jobject,
                                                                    jlong ptr) {
  if (auto p = GetPlayer(ptr)) {
    std::lock_guard<std::mutex> lock(p->mu);
    p->player.Stop();
    p->PresentLocked(env);
    p->DispatchEventsLocked(env);
  }
}

JNIEXPORT jdouble JNICALL Java_com_lynx_animax_AnimaXPlayer_nativeGetDurationMs(
    JNIEnv*, jobject, jlong ptr) {
  if (auto p = GetPlayer(ptr)) {
    std::lock_guard<std::mutex> lock(p->mu);
    return static_cast<jdouble>(p->player.GetDurationMs());
  }
  return 0.0;
}

JNIEXPORT jboolean JNICALL Java_com_lynx_animax_AnimaXPlayer_nativeIsAnimating(
    JNIEnv*, jobject, jlong ptr) {
  if (auto p = GetPlayer(ptr)) {
    std::lock_guard<std::mutex> lock(p->mu);
    return p->player.IsAnimating() ? JNI_TRUE : JNI_FALSE;
  }
  return JNI_FALSE;
}

JNIEXPORT jdouble JNICALL Java_com_lynx_animax_AnimaXPlayer_nativeGetProgress(
    JNIEnv*, jobject, jlong ptr) {
  if (auto p = GetPlayer(ptr)) {
    std::lock_guard<std::mutex> lock(p->mu);
    return static_cast<jdouble>(p->player.GetProgress());
  }
  return 0.0;
}

JNIEXPORT void JNICALL Java_com_lynx_animax_AnimaXPlayer_nativeSeek(JNIEnv* env,
                                                                    jobject,
                                                                    jlong ptr,
                                                                    jint frame) {
  if (auto p = GetPlayer(ptr)) {
    std::lock_guard<std::mutex> lock(p->mu);
    p->player.Seek(static_cast<float>(frame));
    p->PresentLocked(env);
  }
}

JNIEXPORT void JNICALL
Java_com_lynx_animax_AnimaXPlayer_nativeSubscribeUpdateEvent(JNIEnv*, jobject,
                                                             jlong ptr,
                                                             jint frame) {
  if (auto p = GetPlayer(ptr)) {
    std::lock_guard<std::mutex> lock(p->mu);
    p->player.SubscribeUpdateEvent(frame);
  }
}

JNIEXPORT void JNICALL
Java_com_lynx_animax_AnimaXPlayer_nativeUnsubscribeUpdateEvent(JNIEnv*, jobject,
                                                               jlong ptr,
                                                               jint frame) {
  if (auto p = GetPlayer(ptr)) {
    std::lock_guard<std::mutex> lock(p->mu);
    p->player.UnsubscribeUpdateEvent(frame);
  }
}

JNIEXPORT void JNICALL
Java_com_lynx_animax_AnimaXPlayer_nativeSubscribeUpdateEvents(
    JNIEnv* env, jobject, jlong ptr, jintArray frames, jboolean sub) {
  auto p = GetPlayer(ptr);
  if (!p || !frames) return;
  jsize n = env->GetArrayLength(frames);
  jint* elems = env->GetIntArrayElements(frames, nullptr);
  {
    std::lock_guard<std::mutex> lock(p->mu);
    for (jsize i = 0; i < n; ++i) {
      if (sub == JNI_TRUE) p->player.SubscribeUpdateEvent(elems[i]);
      else p->player.UnsubscribeUpdateEvent(elems[i]);
    }
  }
  env->ReleaseIntArrayElements(frames, elems, JNI_ABORT);
}

JNIEXPORT jdouble JNICALL
Java_com_lynx_animax_AnimaXPlayer_nativeGetCurrentFrame(JNIEnv*, jobject,
                                                        jlong ptr) {
  if (auto p = GetPlayer(ptr)) {
    std::lock_guard<std::mutex> lock(p->mu);
    return static_cast<jdouble>(p->player.GetCurrentFrame());
  }
  return 0.0;
}

JNIEXPORT void JNICALL Java_com_lynx_animax_AnimaXPlayer_nativePlaySegment(
    JNIEnv* env, jobject, jlong ptr, jint start_frame, jint end_frame) {
  if (auto p = GetPlayer(ptr)) {
    {
      std::lock_guard<std::mutex> lock(p->mu);
      p->player.PlaySegment(static_cast<float>(start_frame),
                            static_cast<float>(end_frame));
    }
    RequestAndroidVSync(env);
  }
}

JNIEXPORT void JNICALL Java_com_lynx_animax_AnimaXPlayer_nativeDestroy(
    JNIEnv* env, jclass, jlong ptr) {
  if (auto p = GetPlayer(ptr)) {
    UnregisterActivePlayer(p);
    p->Destroy(env);
  }
}

JNIEXPORT void JNICALL Java_com_lynx_animax_AnimaXPlayer_nativeDeletePtr(
    JNIEnv*, jclass, jlong ptr) {
  if (!ptr) return;
  auto* holder = reinterpret_cast<std::shared_ptr<AndroidRustPlayer>*>(ptr);
  delete holder;
}

JNIEXPORT void JNICALL Java_com_lynx_animax_AnimaXPlayer_nativeReload(
    JNIEnv* env, jobject, jlong ptr) {
  if (auto p = GetPlayer(ptr)) {
    bool animating = false;
    {
      std::lock_guard<std::mutex> lock(p->mu);
      p->player.Reload();
      p->PresentLocked(env);
      animating = p->player.IsAnimating();
    }
    if (animating) RequestAndroidVSync(env);
  }
}

JNIEXPORT jobject JNICALL
Java_com_lynx_animax_AnimaXPlayer_nativeGetEventTrackingMap(JNIEnv* env,
                                                            jobject, jlong) {
  jclass map_cls = env->FindClass("com/lynx/animax/base/bridge/JavaOnlyMap");
  jmethodID ctor = env->GetMethodID(map_cls, "<init>", "()V");
  jobject obj = env->NewObject(map_cls, ctor);
  env->DeleteLocalRef(map_cls);
  return obj;
}

JNIEXPORT void JNICALL
Java_com_lynx_animax_AnimaXPlayer_nativeSetFpsEventInterval(JNIEnv*, jobject,
                                                            jlong, jlong) {}

JNIEXPORT void JNICALL Java_com_lynx_animax_AnimaXPlayer_nativeSetMaxFrameRate(
    JNIEnv*, jobject, jlong ptr, jdouble fps) {
  if (auto p = GetPlayer(ptr)) {
    std::lock_guard<std::mutex> lock(p->mu);
    p->player.SetMaxFrameRate(static_cast<float>(fps));
  }
}

JNIEXPORT void JNICALL Java_com_lynx_animax_AnimaXPlayer_nativeOnShow(
    JNIEnv* env, jobject, jlong ptr, jlong state) {
  if (auto p = GetPlayer(ptr)) {
    bool animating = false;
    {
      std::lock_guard<std::mutex> lock(p->mu);
      p->player.OnShow(static_cast<uint8_t>(state));
      animating = p->player.IsAnimating();
    }
    if (animating) RequestAndroidVSync(env);
  }
}

JNIEXPORT void JNICALL Java_com_lynx_animax_AnimaXPlayer_nativeOnHide(
    JNIEnv*, jobject, jlong ptr, jlong state) {
  if (auto p = GetPlayer(ptr)) {
    std::lock_guard<std::mutex> lock(p->mu);
    p->player.OnHide(static_cast<uint8_t>(state));
  }
}

JNIEXPORT void JNICALL
Java_com_lynx_animax_AnimaXPlayer_nativeSetDynamicResource(JNIEnv*, jobject,
                                                           jlong, jboolean) {}

JNIEXPORT void JNICALL Java_com_lynx_animax_AnimaXPlayer_nativeSetMuted(
    JNIEnv*, jobject, jlong, jboolean) {}

JNIEXPORT void JNICALL Java_com_lynx_animax_AnimaXPlayer_nativeSetEnableAudio(
    JNIEnv*, jobject, jlong, jboolean) {}

JNIEXPORT void JNICALL Java_com_lynx_animax_AnimaXPlayer_nativeCancel(
    JNIEnv* env, jobject, jlong ptr) {
  if (auto p = GetPlayer(ptr)) {
    std::lock_guard<std::mutex> lock(p->mu);
    p->player.Stop();
    p->DispatchEventsLocked(env);
  }
}

JNIEXPORT void JNICALL Java_com_lynx_animax_AnimaXPlayer_nativeOnTap(
    JNIEnv* env, jobject, jlong ptr, jfloat x, jfloat y) {
  if (auto p = GetPlayer(ptr)) {
    std::lock_guard<std::mutex> lock(p->mu);
    p->player.OnTap(x, y);
    p->DispatchEventsLocked(env);
  }
}

JNIEXPORT void JNICALL
Java_com_lynx_animax_AnimaXPlayer_nativeUpdateLayerProperty(
    JNIEnv* env, jobject, jlong ptr, jint type, jobject key_path, jobject param,
    jobject callback) {
  auto p = GetPlayer(ptr);
  bool ok = false;
  if (p && param) {
    std::string csv = KeyPathToCsv(env, key_path);
    jclass pcls = env->GetObjectClass(param);
    jint type_idx =
        env->CallIntMethod(param, env->GetMethodID(pcls, "getTypeIndex", "()I"));
    jint target_frame =
        env->CallIntMethod(param, env->GetMethodID(pcls, "getTargetFrame", "()I"));
    std::lock_guard<std::mutex> lock(p->mu);
    // AnimaXValueParam.Type: 0=STRING,1=NUMBER,2=BOOLEAN,3=COORDINATE,4=COLOR
    if (type_idx == 0) {
      auto js = static_cast<jstring>(env->CallObjectMethod(
          param, env->GetMethodID(pcls, "getStringValue", "()Ljava/lang/String;")));
      ok = p->player.SetLayerString(csv, static_cast<uint16_t>(type),
                                    JStringToStdString(env, js), target_frame);
      if (js) env->DeleteLocalRef(js);
    } else if (type_idx == 1) {
      jdouble v = env->CallDoubleMethod(
          param, env->GetMethodID(pcls, "getNumberValue", "()D"));
      ok = p->player.SetLayerNumber(csv, static_cast<uint16_t>(type), v,
                                    target_frame);
    } else if (type_idx == 2) {
      jboolean v = env->CallBooleanMethod(
          param, env->GetMethodID(pcls, "getBooleanValue", "()Z"));
      ok = p->player.SetLayerBool(csv, static_cast<uint16_t>(type),
                                  v == JNI_TRUE, target_frame);
    } else if (type_idx == 3) {
      jdouble x = env->CallDoubleMethod(param, env->GetMethodID(pcls, "getX", "()D"));
      jdouble y = env->CallDoubleMethod(param, env->GetMethodID(pcls, "getY", "()D"));
      jdouble z = env->CallDoubleMethod(param, env->GetMethodID(pcls, "getZ", "()D"));
      ok = p->player.SetLayerCoord(csv, static_cast<uint16_t>(type), x, y, z,
                                   target_frame);
    } else if (type_idx == 4) {
      jint c = env->CallIntMethod(
          param, env->GetMethodID(pcls, "getColorValue", "()I"));
      ok = p->player.SetLayerColor(csv, static_cast<uint16_t>(type), c,
                                   target_frame);
    }
    p->PresentLocked(env);
    env->DeleteLocalRef(pcls);
  }
  if (callback) {
    jclass arr_cls = env->FindClass("com/lynx/animax/base/bridge/JavaOnlyArray");
    jmethodID arr_ctor = env->GetMethodID(arr_cls, "<init>", "()V");
    jmethodID push_str =
        env->GetMethodID(arr_cls, "pushString", "(Ljava/lang/String;)V");
    jobject err_arr = env->NewObject(arr_cls, arr_ctor);
    if (!ok) {
      jstring jmsg = env->NewStringUTF("property update failed");
      env->CallVoidMethod(err_arr, push_str, jmsg);
      env->DeleteLocalRef(jmsg);
    }
    jclass cb_cls = env->GetObjectClass(callback);
    jmethodID mid = env->GetMethodID(
        cb_cls, "onPropertyCallback",
        "(Lcom/lynx/animax/base/bridge/JavaOnlyArray;)V");
    if (mid) {
      env->CallVoidMethod(callback, mid, err_arr);
    }
    env->DeleteLocalRef(cb_cls);
    env->DeleteLocalRef(err_arr);
    env->DeleteLocalRef(arr_cls);
  }
}

JNIEXPORT void JNICALL
Java_com_lynx_animax_AnimaXPlayer_nativeSetResourceProperty(
    JNIEnv* env, jobject, jlong, jint, jstring, jobject, jobject callback) {
  if (callback) {
    jclass arr_cls = env->FindClass("com/lynx/animax/base/bridge/JavaOnlyArray");
    jmethodID arr_ctor = env->GetMethodID(arr_cls, "<init>", "()V");
    jobject err_arr = env->NewObject(arr_cls, arr_ctor);
    jclass cb_cls = env->GetObjectClass(callback);
    jmethodID mid = env->GetMethodID(
        cb_cls, "onPropertyCallback",
        "(Lcom/lynx/animax/base/bridge/JavaOnlyArray;)V");
    if (mid) {
      env->CallVoidMethod(callback, mid, err_arr);
    }
    env->DeleteLocalRef(cb_cls);
    env->DeleteLocalRef(err_arr);
    env->DeleteLocalRef(arr_cls);
  }
}

JNIEXPORT void JNICALL
Java_com_lynx_animax_AnimaXPlayer_nativeAddLayerPropertyCallback(
    JNIEnv*, jobject, jlong, jint, jobject, jobject, jobject) {}

JNIEXPORT void JNICALL
Java_com_lynx_animax_AnimaXPlayer_nativeGetKeysForKeyPath(
    JNIEnv* env, jobject, jlong ptr, jobject key_path, jobject callback) {
  if (!callback) return;
  jclass arr_cls = env->FindClass("com/lynx/animax/base/bridge/JavaOnlyArray");
  jmethodID arr_ctor = env->GetMethodID(arr_cls, "<init>", "()V");
  jmethodID push_arr = env->GetMethodID(
      arr_cls, "pushArray", "(Lcom/lynx/animax/base/bridge/JavaOnlyArray;)V");
  jmethodID push_str =
      env->GetMethodID(arr_cls, "pushString", "(Ljava/lang/String;)V");
  jobject out_arr = env->NewObject(arr_cls, arr_ctor);
  if (auto p = GetPlayer(ptr)) {
    std::string csv = KeyPathToCsv(env, key_path);
    std::string json_str;
    {
      std::lock_guard<std::mutex> lock(p->mu);
      char* json = animax_player_get_keys_for_key_path_json(p->player.raw(),
                                                            csv.c_str());
      if (json) {
        json_str = json;
        animax_string_free(json);
      }
    }
    // Parse JSON array-of-arrays: [["Layer","Group","Fill"], ...]
    size_t i = 0;
    const size_t n = json_str.size();
    int depth = 0;
    jobject cur_inner = nullptr;
    while (i < n) {
      char c = json_str[i];
      if (c == '[') {
        depth++;
        if (depth == 2) {
          cur_inner = env->NewObject(arr_cls, arr_ctor);
        }
        i++;
      } else if (c == ']') {
        if (depth == 2 && cur_inner) {
          env->CallVoidMethod(out_arr, push_arr, cur_inner);
          env->DeleteLocalRef(cur_inner);
          cur_inner = nullptr;
        }
        depth--;
        i++;
      } else if (c == '"' && depth == 2 && cur_inner) {
        i++;
        std::string seg;
        while (i < n && json_str[i] != '"') {
          if (json_str[i] == '\\' && i + 1 < n) {
            i++;
          }
          seg.push_back(json_str[i++]);
        }
        if (i < n) i++;  // closing quote
        jstring jseg = env->NewStringUTF(seg.c_str());
        env->CallVoidMethod(cur_inner, push_str, jseg);
        env->DeleteLocalRef(jseg);
      } else {
        i++;
      }
    }
    if (cur_inner) env->DeleteLocalRef(cur_inner);
  }
  jclass cb_cls = env->GetObjectClass(callback);
  jmethodID mid = env->GetMethodID(
      cb_cls, "onKeyPathListCallback",
      "(Lcom/lynx/animax/base/bridge/JavaOnlyArray;)V");
  if (mid) {
    env->CallVoidMethod(callback, mid, out_arr);
  }
  env->DeleteLocalRef(cb_cls);
  env->DeleteLocalRef(out_arr);
  env->DeleteLocalRef(arr_cls);
}

JNIEXPORT void JNICALL Java_com_lynx_animax_AnimaXPlayer_nativeGetMetricsAsync(
    JNIEnv* env, jobject, jlong, jobject callback) {
  if (!callback) return;
  jclass map_cls = env->FindClass("com/lynx/animax/base/bridge/JavaOnlyMap");
  jmethodID ctor = env->GetMethodID(map_cls, "<init>", "()V");
  jobject metrics = env->NewObject(map_cls, ctor);
  jobject events = env->NewObject(map_cls, ctor);
  jclass cb_cls = env->GetObjectClass(callback);
  jmethodID mid = env->GetMethodID(
      cb_cls, "onMetricsReady",
      "(Lcom/lynx/animax/base/bridge/ReadableMap;"
      "Lcom/lynx/animax/base/bridge/ReadableMap;)V");
  if (mid) {
    env->CallVoidMethod(callback, mid, metrics, events);
  }
  env->DeleteLocalRef(cb_cls);
  env->DeleteLocalRef(events);
  env->DeleteLocalRef(metrics);
  env->DeleteLocalRef(map_cls);
}

JNIEXPORT jlong JNICALL
Java_com_lynx_animax_AnimaXPlayer_nativeGetMemoryUsageBytes(JNIEnv*, jobject,
                                                            jlong ptr) {
  if (auto p = GetPlayer(ptr)) {
    std::lock_guard<std::mutex> lock(p->mu);
    return static_cast<jlong>(
        animax_player_get_memory_usage_bytes(p->player.raw()));
  }
  return 0;
}

// ---- com.lynx.animax.composition.AnimaXCompositionFactory & AnimaXComposition ----
JNIEXPORT void JNICALL
Java_com_lynx_animax_composition_AnimaXCompositionFactory_nativeCreate(
    JNIEnv* env, jobject, jstring scope_name, jlong holder_ptr) {
  std::lock_guard<std::mutex> lock(g_factory_mu);
  g_factory_loaders[JStringToStdString(env, scope_name)] =
      reinterpret_cast<LoaderHolder*>(holder_ptr);
}

JNIEXPORT void JNICALL
Java_com_lynx_animax_composition_AnimaXCompositionFactory_nativeRelease(
    JNIEnv*, jobject) {
  std::lock_guard<std::mutex> lock(g_factory_mu);
  g_factory_loaders.clear();
}

JNIEXPORT void JNICALL
Java_com_lynx_animax_composition_AnimaXCompositionFactory_nativeLoadJson(
    JNIEnv* env, jobject, jstring, jstring json, jfloat density,
    jobject callback) {
  std::string s = JStringToStdString(env, json);
  lynx::animax::RustPlayerBridge probe(density);
  bool ok = probe.SetJson(s);
  jclass cb_cls = env->GetObjectClass(callback);
  jmethodID cb_mid = env->GetMethodID(
      cb_cls, "onCompositionCallback",
      "(Lcom/lynx/animax/composition/AnimaXComposition;Ljava/lang/String;)V");
  if (ok) {
    auto* handle = new CompositionHandle();
    handle->json = std::move(s);
    handle->start_frame =
        static_cast<int64_t>(animax_player_get_start_frame(probe.raw()));
    handle->end_frame =
        static_cast<int64_t>(animax_player_get_end_frame(probe.raw()));
    jclass comp_cls =
        env->FindClass("com/lynx/animax/composition/AnimaXComposition");
    jmethodID comp_create = env->GetStaticMethodID(
        comp_cls, "create",
        "(J)Lcom/lynx/animax/composition/AnimaXComposition;");
    jobject comp_obj = env->CallStaticObjectMethod(
        comp_cls, comp_create, reinterpret_cast<jlong>(handle));
    env->CallVoidMethod(callback, cb_mid, comp_obj, nullptr);
    env->DeleteLocalRef(comp_obj);
    env->DeleteLocalRef(comp_cls);
  } else {
    jstring jerr = env->NewStringUTF("Invalid Lottie JSON");
    env->CallVoidMethod(callback, cb_mid, nullptr, jerr);
    env->DeleteLocalRef(jerr);
  }
  env->DeleteLocalRef(cb_cls);
}

JNIEXPORT void JNICALL
Java_com_lynx_animax_composition_AnimaXCompositionFactory_nativeLoadUri(
    JNIEnv* env, jobject, jstring scope_name, jstring uri, jfloat density,
    jobject callback) {
  std::string scope = JStringToStdString(env, scope_name);
  std::string uri_str = JStringToStdString(env, uri);
  LoaderHolder* holder = nullptr;
  {
    std::lock_guard<std::mutex> lock(g_factory_mu);
    auto it = g_factory_loaders.find(scope);
    if (it != g_factory_loaders.end()) holder = it->second;
  }
  jobject cb_global = env->NewGlobalRef(callback);
  DispatchUriLoad(
      env, holder, uri_str,
      [cb_global, density](JNIEnv* cb_env, const uint8_t* data, size_t len,
                           const std::string& file_path,
                           const std::string& err) {
        jclass cb_cls = cb_env->GetObjectClass(cb_global);
        jmethodID cb_mid = cb_env->GetMethodID(
            cb_cls, "onCompositionCallback",
            "(Lcom/lynx/animax/composition/AnimaXComposition;Ljava/lang/String;)V");
        std::string file_data;
        const uint8_t* bytes = data;
        size_t byte_len = len;
        if ((!bytes || byte_len == 0) && !file_path.empty()) {
          file_data = ReadFileToString(file_path);
          bytes = reinterpret_cast<const uint8_t*>(file_data.data());
          byte_len = file_data.size();
        }
        bool ok = false;
        auto* handle = new CompositionHandle();
        lynx::animax::RustPlayerBridge probe(density);
        if (err.empty() && bytes && byte_len > 0) {
          if (byte_len >= 4 && bytes[0] == 0x50 && bytes[1] == 0x4B &&
              bytes[2] == 0x03 && bytes[3] == 0x04) {
            handle->is_zip = true;
            handle->zip_bytes.assign(bytes, bytes + byte_len);
            ok = probe.SetZip(bytes, byte_len);
          } else {
            handle->json.assign(reinterpret_cast<const char*>(bytes), byte_len);
            ok = probe.SetJson(handle->json);
          }
        }
        if (ok) {
          handle->start_frame =
              static_cast<int64_t>(animax_player_get_start_frame(probe.raw()));
          handle->end_frame =
              static_cast<int64_t>(animax_player_get_end_frame(probe.raw()));
          jclass comp_cls =
              cb_env->FindClass("com/lynx/animax/composition/AnimaXComposition");
          jmethodID comp_create = cb_env->GetStaticMethodID(
              comp_cls, "create",
              "(J)Lcom/lynx/animax/composition/AnimaXComposition;");
          jobject comp_obj = cb_env->CallStaticObjectMethod(
              comp_cls, comp_create, reinterpret_cast<jlong>(handle));
          cb_env->CallVoidMethod(cb_global, cb_mid, comp_obj, nullptr);
          cb_env->DeleteLocalRef(comp_obj);
          cb_env->DeleteLocalRef(comp_cls);
        } else {
          delete handle;
          jstring jerr = cb_env->NewStringUTF(
              err.empty() ? "Failed to parse composition" : err.c_str());
          cb_env->CallVoidMethod(cb_global, cb_mid, nullptr, jerr);
          cb_env->DeleteLocalRef(jerr);
        }
        cb_env->DeleteLocalRef(cb_cls);
        cb_env->DeleteGlobalRef(cb_global);
      });
}

JNIEXPORT jlong JNICALL
Java_com_lynx_animax_composition_AnimaXComposition_nativeGetStartFrame(
    JNIEnv*, jobject, jlong ptr) {
  auto* c = reinterpret_cast<CompositionHandle*>(ptr);
  return c ? c->start_frame : 0;
}

JNIEXPORT jlong JNICALL
Java_com_lynx_animax_composition_AnimaXComposition_nativeGetEndFrame(
    JNIEnv*, jobject, jlong ptr) {
  auto* c = reinterpret_cast<CompositionHandle*>(ptr);
  return c ? c->end_frame : 0;
}

JNIEXPORT void JNICALL
Java_com_lynx_animax_composition_AnimaXComposition_nativeDestroy(JNIEnv*,
                                                                 jclass,
                                                                 jlong ptr) {
  delete reinterpret_cast<CompositionHandle*>(ptr);
}

JNIEXPORT void JNICALL
Java_com_lynx_animax_player_NativePlayerCallback_nativeOnError(JNIEnv*, jclass,
                                                               jlong, jstring) {}

}  // extern "C"
