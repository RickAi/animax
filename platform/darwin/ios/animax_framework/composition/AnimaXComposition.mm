// Copyright 2025 The Lynx Authors. All rights reserved.
// Licensed under the Apache License Version 2.0 that can be found in the
// LICENSE file in the root directory of this source tree.

#import <AnimaX/AnimaXComposition.h>
#import "AnimaX/AnimaXComposition+Internal.h"
#import "AnimaXRustCoreIOS.h"
#if !ANIMAX_USE_RUST_CORE
#include "src/model/composition_model.h"
#endif

@implementation AnimaXComposition {
#if ANIMAX_USE_RUST_CORE
  std::shared_ptr<lynx::animax::RustCompositionModel> _rustModel;
#else
  std::shared_ptr<lynx::animax::CompositionModel> _model;
#endif
}

- (instancetype)initWithSharedModel:(void *)sharedModelPtr {
  self = [super init];
  if (self) {
#if ANIMAX_USE_RUST_CORE
    auto modelPtr =
        reinterpret_cast<std::shared_ptr<lynx::animax::RustCompositionModel> *>(sharedModelPtr);
    if (modelPtr) {
      _rustModel = *modelPtr;
    }
#else
    auto modelPtr =
        reinterpret_cast<std::shared_ptr<lynx::animax::CompositionModel> *>(sharedModelPtr);
    if (modelPtr) {
      _model = *modelPtr;
    }
#endif
  }
  return self;
}

- (CGFloat)startFrame {
#if ANIMAX_USE_RUST_CORE
  return _rustModel ? _rustModel->start_frame : 0;
#else
  if (!_model) {
    return 0;
  }
  return _model->GetStartFrame();
#endif
}

- (CGFloat)endFrame {
#if ANIMAX_USE_RUST_CORE
  return _rustModel ? _rustModel->end_frame : 0;
#else
  if (!_model) {
    return 0;
  }
  return _model->GetTimelineEndFrame();
#endif
}

- (void *)internalModel {
#if ANIMAX_USE_RUST_CORE
  return _rustModel ? static_cast<void *>(&_rustModel) : nullptr;
#else
  if (!_model) {
    return nullptr;
  }
  return static_cast<void *>(&_model);
#endif
}

@end
