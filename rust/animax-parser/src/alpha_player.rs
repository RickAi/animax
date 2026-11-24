// Copyright 2026 The Lynx Authors. All rights reserved.
// Licensed under the Apache License Version 2.0 that can be found in the
// LICENSE file in the root directory of this source tree.

use crate::lottie::{get_i32, get_str, ParseError};
use animax_model::*;
use serde_json::Value;

/// Parses an AlphaPlayer `config.json` string into a `CompositionModel` with an `AlphaVideo` layer.
pub fn parse_alpha_player_json(json: &str) -> Result<CompositionModel, ParseError> {
    let root: Value =
        serde_json::from_str(json).map_err(|e| ParseError(format!("invalid json: {e}")))?;
    parse_alpha_player_value(&root)
}

pub(crate) fn parse_alpha_player_value(root: &Value) -> Result<CompositionModel, ParseError> {
    let portrait = root.get("portrait").map(parse_item);
    let landscape = root.get("landscape").map(parse_item);

    let active = portrait
        .as_ref()
        .or(landscape.as_ref())
        .ok_or_else(|| ParseError("alpha player config missing portrait/landscape".into()))?;

    let frame_count = (active.total_frames.max(1)) as f32;
    let w = active.actual_width.max(active.rgb_frame.width() as i32).max(1) as f32;
    let h = active.actual_height.max(active.rgb_frame.height() as i32).max(1) as f32;

    let mut comp = CompositionModel {
        name: "AlphaPlayerComposition".to_string(),
        bounds: RectF::from_wh(w, h),
        start_frame: 0.0,
        end_frame: frame_count,
        frame_rate: 30.0,
        alpha_player_config: Some(AlphaPlayerConfig {
            portrait: portrait.clone(),
            landscape: landscape.clone(),
        }),
        ..Default::default()
    };

    comp.videos.insert(
        "0".to_string(),
        VideoAsset {
            id: "0".to_string(),
            width: active.video_width,
            height: active.video_height,
            file_name: active.path.clone(),
            dir_name: String::new(),
            alpha_layout: AlphaVideoLayout::Custom,
        },
    );

    comp.layers.push(LayerModel {
        name: "AlphaPlayerTransformed".to_string(),
        layer_type: LayerType::AlphaVideo,
        id: 1,
        parent_id: -1,
        ref_id: "0".to_string(),
        time_stretch: 1.0,
        start_frame: 0.0,
        in_frame: 0.0,
        out_frame: frame_count,
        ..Default::default()
    });

    Ok(comp)
}

fn parse_item(v: &Value) -> AlphaPlayerItem {
    let rgb = parse_rect4(v.get("rgbFrame"));
    let alpha = parse_rect4(v.get("aFrame"));
    let vw = get_i32(v, "videoW", 0);
    let vh = get_i32(v, "videoH", 0);
    let aw = get_i32(v, "w", rgb.width() as i32);
    let ah = get_i32(v, "h", rgb.height() as i32);

    AlphaPlayerItem {
        path: get_str(v, "path"),
        align: get_i32(v, "align", 0),
        version: get_i32(v, "v", 1),
        total_frames: get_i32(v, "f", 60),
        video_width: vw,
        video_height: vh,
        actual_width: aw,
        actual_height: ah,
        rgb_frame: if rgb.is_empty() && vw > 0 && vh > 0 {
            RectF::new(0.0, 0.0, (vw / 2) as f32, vh as f32)
        } else {
            rgb
        },
        alpha_frame: if alpha.is_empty() && vw > 0 && vh > 0 {
            RectF::new((vw / 2) as f32, 0.0, vw as f32, vh as f32)
        } else {
            alpha
        },
    }
}

fn parse_rect4(v: Option<&Value>) -> RectF {
    let Some(arr) = v.and_then(|x| x.as_array()) else {
        return RectF::default();
    };
    if arr.len() < 4 {
        return RectF::default();
    }
    let x = arr[0].as_f64().unwrap_or(0.0) as f32;
    let y = arr[1].as_f64().unwrap_or(0.0) as f32;
    let w = arr[2].as_f64().unwrap_or(0.0) as f32;
    let h = arr[3].as_f64().unwrap_or(0.0) as f32;
    RectF::new(x, y, x + w, y + h)
}
