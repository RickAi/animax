// Copyright 2026 The Lynx Authors. All rights reserved.
// Licensed under the Apache License Version 2.0 that can be found in the
// LICENSE file in the root directory of this source tree.

//! KeyPath resolution and dynamic property update system matching `src/property/`.

pub mod key_path;
pub mod property_type;
pub mod request;
pub mod value_param;

pub use key_path::AnimaXKeyPath;
pub use property_type::{LayerPropertyType, ResourcePropertyType};
pub use request::{PropertyOverride, PropertyStore, PropertyUpdateRequest, PropertyUpdateResponse};
pub use value_param::{AnimaXValueParam, ColorFilterParam, ValueParamData};
