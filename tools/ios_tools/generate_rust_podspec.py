#!/usr/bin/env python3
# Copyright 2026 The Lynx Authors. All rights reserved.
# Licensed under the Apache License Version 2.0 that can be found in the
# LICENSE file in the root directory of this source tree.

"""Generates a self-contained CocoaPods `AnimaX.podspec` backed by the pure Rust core & Rust Skity."""

import argparse
import os

ROOT = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))

PODSPEC_TEMPLATE = """Pod::Spec.new do |s|
  s.name             = 'AnimaX'
  s.version          = '1.0.0'
  s.summary          = 'AnimaX pure Rust core & Rust Skity animation framework for iOS.'
  s.homepage         = 'https://github.com/lynx-family/animax'
  s.license          = { :type => 'Apache 2.0' }
  s.author           = 'Lynx'
  s.source           = { :git => 'https://github.com/lynx-family/animax.git', :tag => s.version.to_s }
  s.ios.deployment_target = '12.0'
  s.requires_arc     = true
  s.default_subspecs = ['Core', 'Framework']

  s.pod_target_xcconfig = {
    'CLANG_CXX_LANGUAGE_STANDARD' => 'gnu++17',
    'GCC_PREPROCESSOR_DEFINITIONS' => '$(inherited) OS_IOS=1 ANIMAX_USE_RUST_CORE=1',
    'HEADER_SEARCH_PATHS' => '"$(PODS_TARGET_SRCROOT)" "$(PODS_TARGET_SRCROOT)/include" "$(PODS_TARGET_SRCROOT)/platform/darwin/ios/animax_framework" "$(PODS_TARGET_SRCROOT)/platform/darwin/ios/animax_framework/util"',
  }

  s.subspec 'Core' do |core|
    core.source_files = [
      'include/animax_rust_ffi.h',
      'src/rust_bridge/animax_rust_bridge.h',
      'src/rust_bridge/animax_rust_bridge.cc',
    ]
    core.public_header_files = ['include/animax_rust_ffi.h']
    core.vendored_frameworks = 'platform/darwin/ios/rust_libs/AnimaXRustCore.xcframework'
    core.frameworks = ['CoreGraphics', 'CoreVideo', 'QuartzCore', 'UIKit', 'VideoToolbox', 'Metal']
  end

  s.subspec 'Framework' do |fw|
    fw.dependency 'AnimaX/Core'
    fw.source_files = [
      'platform/darwin/ios/animax_framework/**/*.{h,m,mm}',
    ]
    fw.exclude_files = [
      'platform/darwin/ios/animax_framework/**/*UnitTest*.{h,m,mm}',
    ]
    fw.public_header_files = [
      'platform/darwin/ios/animax_framework/AnimaXAnimationListener.h',
      'platform/darwin/ios/animax_framework/AnimaXContext.h',
      'platform/darwin/ios/animax_framework/AnimaXImageView.h',
      'platform/darwin/ios/animax_framework/AnimaXPlayer.h',
      'platform/darwin/ios/animax_framework/AnimaXPlayerProtocol.h',
      'platform/darwin/ios/animax_framework/AnimaXView.h',
      'platform/darwin/ios/animax_framework/AnimaXViewProtocol.h',
      'platform/darwin/ios/animax_framework/ability/BaseAnimaXAbility.h',
      'platform/darwin/ios/animax_framework/ability/DefaultAnimaXAbility.h',
      'platform/darwin/ios/animax_framework/composition/AnimaXComposition.h',
      'platform/darwin/ios/animax_framework/composition/AnimaXCompositionFactory.h',
      'platform/darwin/ios/animax_framework/loader/AnimaXImageDecoderService.h',
      'platform/darwin/ios/animax_framework/loader/AnimaXLoaderProtocol.h',
      'platform/darwin/ios/animax_framework/loader/AnimaXLoaderRequest.h',
      'platform/darwin/ios/animax_framework/loader/AnimaXLoaderResponse.h',
      'platform/darwin/ios/animax_framework/loader/AnimaXResourceFactoryService.h',
      'platform/darwin/ios/animax_framework/monitor/AnimaXMonitorAbilityDelegate.h',
      'platform/darwin/ios/animax_framework/monitor/AnimaXMonitorService.h',
      'platform/darwin/ios/animax_framework/property/AnimaXFrameInfo.h',
      'platform/darwin/ios/animax_framework/property/AnimaXKeyPath.h',
      'platform/darwin/ios/animax_framework/property/AnimaXKeyPathListCallback.h',
      'platform/darwin/ios/animax_framework/property/AnimaXPropertyCallback.h',
      'platform/darwin/ios/animax_framework/property/AnimaXValueCallback.h',
      'platform/darwin/ios/animax_framework/property/AnimaXValueParam.h',
      'platform/darwin/ios/animax_framework/property/LayerPropertyType.h',
      'platform/darwin/ios/animax_framework/property/ResourcePropertyType.h',
      'platform/darwin/ios/animax_framework/service/AnimaXService.h',
      'platform/darwin/ios/animax_framework/service/AnimaXServiceCenter.h',
      'platform/darwin/ios/animax_framework/service/AnimaXServiceRegistry.h',
      'platform/darwin/ios/animax_framework/service/AnimaXServiceScope.h',
      'platform/darwin/ios/animax_framework/surface/AnimaXRenderTypes.h',
      'platform/darwin/ios/animax_framework/surface/AnimaXSurfaceDrawable.h',
      'platform/darwin/ios/animax_framework/util/AnimaX.h',
      'platform/darwin/ios/animax_framework/util/AnimaXScopedObject.h',
      'platform/darwin/ios/animax_framework/util/AnimaXURLUtils.h',
    ]
  end
end
"""


def main():
    parser = argparse.ArgumentParser(description="Generate pure Rust AnimaX.podspec")
    parser.add_argument(
        "--output",
        default=os.path.join(ROOT, "AnimaX.podspec"),
        help="Output path for AnimaX.podspec",
    )
    args = parser.parse_args()
    with open(args.output, "w", encoding="utf-8") as f:
        f.write(PODSPEC_TEMPLATE)
    print(f"Generated pure Rust podspec at {args.output}")


if __name__ == "__main__":
    main()
