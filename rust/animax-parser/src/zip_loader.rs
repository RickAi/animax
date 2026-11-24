// Copyright 2026 The Lynx Authors. All rights reserved.
// Licensed under the Apache License Version 2.0 that can be found in the
// LICENSE file in the root directory of this source tree.

use crate::lottie::{parse_lottie_json, ParseError};
use animax_model::CompositionModel;
use std::collections::HashMap;
use std::io::{Cursor, Read};

/// Unpacked ZIP bundle containing the parsed `CompositionModel` and extracted asset bytes.
pub struct ZipBundle {
    pub composition: CompositionModel,
    pub files: HashMap<String, Vec<u8>>,
}

/// Unpacks an AnimaX `.zip` bundle (Lottie JSON + images/fonts/videos or AlphaPlayer config.json).
pub fn parse_zip_bundle(bytes: &[u8]) -> Result<ZipBundle, ParseError> {
    let cursor = Cursor::new(bytes);
    let mut archive = zip::ZipArchive::new(cursor)
        .map_err(|e| ParseError(format!("invalid zip archive: {e}")))?;

    let mut files: HashMap<String, Vec<u8>> = HashMap::new();
    let mut json_candidate: Option<String> = None;

    for i in 0..archive.len() {
        let mut entry = archive
            .by_index(i)
            .map_err(|e| ParseError(format!("zip entry error: {e}")))?;
        if entry.is_dir() {
            continue;
        }
        let name = entry.name().to_string();
        if name.starts_with("__MACOSX/") {
            continue;
        }
        let mut buf = Vec::with_capacity(entry.size() as usize);
        entry
            .read_to_end(&mut buf)
            .map_err(|e| ParseError(format!("zip read error: {e}")))?;

        let lower = name.to_ascii_lowercase();
        if lower.ends_with(".json") && (json_candidate.is_none() || lower.ends_with("config.json") || lower.ends_with("data.json")) {
            if let Ok(text) = String::from_utf8(buf.clone()) {
                json_candidate = Some(text);
            }
        }
        files.insert(name, buf);
    }

    let json_text = json_candidate
        .ok_or_else(|| ParseError("zip bundle contains no .json composition".into()))?;
    let mut composition = parse_lottie_json(&json_text)?;

    // Attach embedded bytes from ZIP into matching ImageAssets
    for asset in composition.images.values_mut() {
        if asset.embedded_data.is_some() {
            continue;
        }
        let full = format!("{}{}", asset.dir_name, asset.file_name);
        if let Some(data) = files
            .get(&full)
            .or_else(|| files.get(&asset.file_name))
            .or_else(|| {
                files
                    .iter()
                    .find(|(k, _)| k.ends_with(&asset.file_name))
                    .map(|(_, v)| v)
            })
        {
            asset.embedded_data = Some(data.clone());
        }
    }

    Ok(ZipBundle { composition, files })
}
