// Copyright 2026 The Lynx Authors. All rights reserved.
// Licensed under the Apache License Version 2.0 that can be found in the
// LICENSE file in the root directory of this source tree.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum UriScheme {
    Http,
    Https,
    File,
    Asset,
    Data,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UriInfo {
    pub raw: String,
    pub scheme: UriScheme,
    pub path: String,
    pub extension: String,
}

/// Validates whether a URL uses an AnimaX-supported scheme (`http`, `asset`, `file`).
pub fn is_url_valid(url: &str) -> bool {
    url.starts_with("http") || url.starts_with("asset") || url.starts_with("file")
}

pub fn parse_uri(uri: &str) -> UriInfo {
    let (scheme, rest) = if let Some(r) = uri.strip_prefix("https://") {
        (UriScheme::Https, r)
    } else if let Some(r) = uri.strip_prefix("http://") {
        (UriScheme::Http, r)
    } else if let Some(r) = uri.strip_prefix("file://") {
        (UriScheme::File, r)
    } else if let Some(r) = uri.strip_prefix("asset://") {
        (UriScheme::Asset, r)
    } else if let Some(r) = uri.strip_prefix("data:") {
        (UriScheme::Data, r)
    } else {
        (UriScheme::Unknown, uri)
    };

    let path_part = rest.split(['?', '#']).next().unwrap_or("");
    let extension = path_part
        .rsplit('/')
        .next()
        .and_then(|file| file.rsplit_once('.'))
        .map(|(_, ext)| ext.to_ascii_lowercase())
        .unwrap_or_default();

    UriInfo {
        raw: uri.to_string(),
        scheme,
        path: path_part.to_string(),
        extension,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_uri_parsing() {
        assert!(is_url_valid("https://example.com/a.json"));
        assert!(is_url_valid("asset://anim/test.zip"));
        assert!(is_url_valid("file:///tmp/a.json"));
        assert!(!is_url_valid("ftp://example.com/a.json"));

        let info = parse_uri("https://cdn.example.com/lottie/demo.ZIP?v=1#hash");
        assert_eq!(info.scheme, UriScheme::Https);
        assert_eq!(info.extension, "zip");
    }
}
