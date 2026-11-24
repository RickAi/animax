// Copyright 2026 The Lynx Authors. All rights reserved.
// Licensed under the Apache License Version 2.0 that can be found in the
// LICENSE file in the root directory of this source tree.

//! Base utilities, color parsing, math interpolation, event definitions,
//! visibility flags, object-fit layout calculations, and URI helpers for AnimaX.

pub mod color;
pub mod event;
pub mod fit;
pub mod math;
pub mod uri;
pub mod visibility;

pub use color::{Color, ColorUtil};
pub use event::{Event, EventError, EventKeys, EventParamMap, EventParamValue, EventWarning};
pub use fit::{compute_fit_transform, FitTransform, LayerBoundsSpace, ObjectFit, ObjectPosition};
pub use math::{floor_mod, gamma_evaluate, gamma_evaluate_color, lerp_f32, lerp_i32};
pub use uri::{is_url_valid, parse_uri, UriInfo, UriScheme};
pub use visibility::{
    stringify_visibility_state, stringify_visibility_state_flag, VisibilityState,
    VisibilityStateFlags,
};
