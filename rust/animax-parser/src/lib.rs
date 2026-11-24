// Copyright 2026 The Lynx Authors. All rights reserved.
// Licensed under the Apache License Version 2.0 that can be found in the
// LICENSE file in the root directory of this source tree.

//! Pure Rust Lottie JSON, AlphaPlayer config.json, and ZIP composition parser
//! replacing RapidJSON and `src/parser/`.

pub mod alpha_player;
pub mod lottie;
pub mod zip_loader;

pub use alpha_player::parse_alpha_player_json;
pub use lottie::{parse_lottie_json, ParseError};
pub use zip_loader::{parse_zip_bundle, ZipBundle};
