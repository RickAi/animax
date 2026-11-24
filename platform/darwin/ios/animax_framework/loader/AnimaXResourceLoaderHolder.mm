// Copyright 2025 The Lynx Authors. All rights reserved.
// Licensed under the Apache License Version 2.0 that can be found in the
// LICENSE file in the root directory of this source tree.

#import "AnimaXResourceLoaderHolder.h"
#import <AnimaX/AnimaXContext.h>
#import <AnimaX/AnimaXServiceCenter.h>
#import <AnimaX/AnimaXServiceScope.h>
#import "AnimaXContext+Internal.h"
#import "AnimaXResourceLoaderHolder+Internal.h"
#import "AnimaXRustCoreIOS.h"
#if !ANIMAX_USE_RUST_CORE
#import "AnimaXFileLoader.h"
#include "include/player/animax_player_builder.h"
#include "src/resource/composition_loader/animax_composition_factory.h"
#include "src/resource/resource_loader/ios/resource_pipeline_ios.h"
#include "src/resource/resource_loader/ios/unzip_loader_ios.h"
#endif

@interface AnimaXResourceLoaderHolder ()
#if ANIMAX_USE_RUST_CORE
@property(nonatomic, strong, nullable) NSBundle *assetBundle;
#endif
@end

@implementation AnimaXResourceLoaderHolder {
#if !ANIMAX_USE_RUST_CORE
  std::shared_ptr<lynx::animax::ResourcePipelineIOS> _resourcePipeline;
  std::shared_ptr<lynx::animax::UnzipLoaderIOS> _unzipLoader;
#endif
}

- (instancetype)initPrivate:(AnimaXContext *)context {
  self = [super init];
  if (self) {
#if !ANIMAX_USE_RUST_CORE
    _resourcePipeline = lynx::animax::ResourcePipelineIOS::MakePipeline(context);
    _unzipLoader =
        std::shared_ptr<lynx::animax::UnzipLoaderIOS>{new lynx::animax::UnzipLoaderIOS{}};
#else
    (void)context;
#endif
  }
  return self;
}

+ (AnimaXResourceLoaderHolder *)loaderForPlayer:(void *)builderPtr
                                        context:(AnimaXContext *)context {
#if ANIMAX_USE_RUST_CORE
  (void)builderPtr;
  return [[AnimaXResourceLoaderHolder alloc] initPrivate:context];
#else
  if (!builderPtr) {
    return nil;
  }
  auto builder = static_cast<lynx::animax::AnimaXPlayerBuilder *>(builderPtr);
  if (!builder) {
    return nil;
  }
  AnimaXResourceLoaderHolder *loader = [[AnimaXResourceLoaderHolder alloc] initPrivate:context];
  builder->SetResourceLoader(loader->_resourcePipeline).SetUnzipLoader(loader->_unzipLoader);
  return loader;
#endif
}

+ (AnimaXResourceLoaderHolder *)loaderForCompositionFactory:(AnimaXContext *)context {
  AnimaXResourceLoaderHolder *loader = [[AnimaXResourceLoaderHolder alloc] initPrivate:context];
#if !ANIMAX_USE_RUST_CORE
  AnimaXServiceScope *scope = context.ability.getServiceScope;
  lynx::animax::AnimaXCompositionFactory::Instance().CreateLoader(
      scope.description.UTF8String, loader->_resourcePipeline, loader->_unzipLoader);
#endif
  return loader;
}

- (void)dealloc {
#if !ANIMAX_USE_RUST_CORE
  _resourcePipeline.reset();
  _unzipLoader.reset();
#endif
}

- (void)setAssetBundle:(NSBundle *)bundle {
#if ANIMAX_USE_RUST_CORE
  _assetBundle = bundle;
#else
  AnimaXFileLoader *fileLoader = _resourcePipeline->GetFileLoader();
  [fileLoader setAssetBundle:bundle];
#endif
}
@end
