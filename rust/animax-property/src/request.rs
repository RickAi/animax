// Copyright 2026 The Lynx Authors. All rights reserved.
// Licensed under the Apache License Version 2.0 that can be found in the
// LICENSE file in the root directory of this source tree.

use crate::key_path::AnimaXKeyPath;
use crate::property_type::{LayerPropertyType, ResourcePropertyType};
use crate::value_param::AnimaXValueParam;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PropertyOverride {
    pub key_path: AnimaXKeyPath,
    pub property_type: LayerPropertyType,
    pub value: AnimaXValueParam,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct PropertyUpdateRequest {
    pub layer_updates: Vec<PropertyOverride>,
    pub resource_updates: Vec<(String, ResourcePropertyType, AnimaXValueParam)>,
}

impl PropertyUpdateRequest {
    pub fn add_layer_property(
        &mut self,
        key_path: AnimaXKeyPath,
        property_type: LayerPropertyType,
        value: AnimaXValueParam,
    ) {
        self.layer_updates.push(PropertyOverride {
            key_path,
            property_type,
            value,
        });
    }

    pub fn add_resource_property(
        &mut self,
        asset_id: impl Into<String>,
        property_type: ResourcePropertyType,
        value: AnimaXValueParam,
    ) {
        self.resource_updates
            .push((asset_id.into(), property_type, value));
    }
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct PropertyUpdateResponse {
    pub success: bool,
    pub updated_count: usize,
    pub error_message: Option<String>,
}

/// Runtime store holding active dynamic property overrides queried during frame evaluation.
#[derive(Debug, Clone, Default)]
pub struct PropertyStore {
    overrides: Vec<PropertyOverride>,
}

impl PropertyStore {
    pub fn new() -> Self {
        Self {
            overrides: Vec::new(),
        }
    }

    pub fn apply_override(&mut self, item: PropertyOverride) {
        if let Some(existing) = self.overrides.iter_mut().find(|o| {
            o.key_path == item.key_path
                && o.property_type == item.property_type
                && o.value.target_frame == item.value.target_frame
        }) {
            existing.value = item.value;
        } else {
            self.overrides.push(item);
        }
    }

    pub fn clear(&mut self) {
        self.overrides.clear();
    }

    /// Resolves the latest matching override for a concrete element path, property type, and current frame.
    pub fn resolve(
        &self,
        path: &[&str],
        property_type: LayerPropertyType,
        current_frame: i32,
    ) -> Option<&AnimaXValueParam> {
        let mut fallback: Option<&AnimaXValueParam> = None;
        for o in self.overrides.iter().rev() {
            if o.property_type != property_type {
                continue;
            }
            if !o.key_path.matches_full_path(path) {
                continue;
            }
            if o.value.has_target_frame() {
                if o.value.target_frame == current_frame {
                    return Some(&o.value);
                }
            } else if fallback.is_none() {
                fallback = Some(&o.value);
            }
        }
        fallback
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use animax_base::Color;

    #[test]
    fn property_store_resolution_and_frame_targeting() {
        let mut store = PropertyStore::new();
        store.apply_override(PropertyOverride {
            key_path: AnimaXKeyPath::from_slices(&["**", "Fill 1"]),
            property_type: LayerPropertyType::Color,
            value: AnimaXValueParam::from_color(
                Color::pack_argb(255, 255, 0, 0),
                AnimaXValueParam::FRAME_INDEX_ALL,
            ),
        });
        store.apply_override(PropertyOverride {
            key_path: AnimaXKeyPath::from_slices(&["Layer A", "Group 1", "Fill 1"]),
            property_type: LayerPropertyType::Color,
            value: AnimaXValueParam::from_color(Color::pack_argb(255, 0, 255, 0), 10),
        });

        let c_frame_5 = store
            .resolve(&["Layer A", "Group 1", "Fill 1"], LayerPropertyType::Color, 5)
            .and_then(|v| v.as_color());
        assert_eq!(c_frame_5, Some(Color::from_rgb(255, 0, 0)));

        let c_frame_10 = store
            .resolve(&["Layer A", "Group 1", "Fill 1"], LayerPropertyType::Color, 10)
            .and_then(|v| v.as_color());
        assert_eq!(c_frame_10, Some(Color::from_rgb(0, 255, 0)));
    }
}
