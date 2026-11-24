// Copyright 2026 The Lynx Authors. All rights reserved.
// Licensed under the Apache License Version 2.0 that can be found in the
// LICENSE file in the root directory of this source tree.

//! Pure Rust AnimaX rendering & playback engine replacing `src/content/`,
//! `src/layer/`, and `src/player/`.

pub mod content;
pub mod layer;
pub mod player;

pub use player::{AnimaXPlayer, PlayerEvent};
