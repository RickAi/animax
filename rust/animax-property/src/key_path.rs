// Copyright 2026 The Lynx Authors. All rights reserved.
// Licensed under the Apache License Version 2.0 that can be found in the
// LICENSE file in the root directory of this source tree.

use serde::{Deserialize, Serialize};

/// Hierarchical key path selector supporting `*` wildcard, `**` globstar, and `__container`.
/// Matches `lynx::animax::AnimaXKeyPath` semantics 1:1.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct AnimaXKeyPath {
    keys: Vec<String>,
}

impl AnimaXKeyPath {
    pub const WILDCARD_KEY: &'static str = "*";
    pub const GLOBSTAR_KEY: &'static str = "**";
    pub const CONTAINER_KEY: &'static str = "__container";

    pub fn new(keys: Vec<String>) -> Self {
        Self { keys }
    }

    pub fn from_slices(keys: &[&str]) -> Self {
        Self {
            keys: keys.iter().map(|s| (*s).to_string()).collect(),
        }
    }

    pub fn keys(&self) -> &[String] {
        &self.keys
    }

    pub fn add_key(&self, key: impl Into<String>) -> Self {
        let mut next = self.keys.clone();
        next.push(key.into());
        Self { keys: next }
    }

    pub fn is_container(key: &str) -> bool {
        key == Self::CONTAINER_KEY
    }

    pub fn ends_with_globstar(&self) -> bool {
        self.keys
            .last()
            .is_some_and(|k| k == Self::GLOBSTAR_KEY)
    }

    pub fn matches(&self, key: &str, depth: i32) -> bool {
        if Self::is_container(key) {
            return true;
        }
        if depth < 0 || (depth as usize) >= self.keys.len() {
            return false;
        }
        let key_at_depth = &self.keys[depth as usize];
        key_at_depth == key
            || key_at_depth == Self::GLOBSTAR_KEY
            || key_at_depth == Self::WILDCARD_KEY
    }

    pub fn increment_depth_by(&self, key: &str, depth: i32) -> i32 {
        if Self::is_container(key) {
            return 0;
        }
        let size = self.keys.len();
        if depth < 0 || size == 0 || (depth as usize) >= size {
            return 1;
        }
        let d = depth as usize;
        if self.keys[d] != Self::GLOBSTAR_KEY {
            return 1;
        }
        if d == size - 1 {
            return 0;
        }
        if self.keys[d + 1] == key {
            return 2;
        }
        0
    }

    pub fn fully_resolves_to(&self, key: &str, depth: i32) -> bool {
        let size = self.keys.len();
        if depth < 0 || (depth as usize) >= size {
            return false;
        }
        let d = depth as usize;
        let is_last_depth = d == size - 1;
        let key_at_depth = &self.keys[d];
        let is_globstar = key_at_depth == Self::GLOBSTAR_KEY;

        if !is_globstar {
            let matches = key_at_depth == key || key_at_depth == Self::WILDCARD_KEY;
            return (is_last_depth || (d + 2 == size && self.ends_with_globstar())) && matches;
        }

        let is_globstar_next_matches = !is_last_depth && self.keys[d + 1] == key;
        if is_globstar_next_matches {
            return d + 2 == size || (d + 3 == size && self.ends_with_globstar());
        }

        if is_last_depth {
            return true;
        }
        if d + 1 < size - 1 {
            return false;
        }
        if self.keys[d + 1] == Self::GLOBSTAR_KEY {
            return true;
        }
        self.keys[d + 1] == key
    }

    pub fn propagate_to_children(&self, key: &str, depth: i32) -> bool {
        if depth < 0 || key == Self::CONTAINER_KEY {
            return true;
        }
        if self.keys.is_empty() {
            return false;
        }
        let d = depth as usize;
        d < self.keys.len() - 1 || (d < self.keys.len() && self.keys[d] == Self::GLOBSTAR_KEY)
    }

    pub fn description(&self) -> String {
        self.keys.join(",")
    }

    /// Evaluates whether this selector matches a full concrete element path (e.g., `["Layer1", "Group1", "Fill 1"]`).
    pub fn matches_full_path(&self, path: &[&str]) -> bool {
        let mut depth: i32 = 0;
        for (idx, &seg) in path.iter().enumerate() {
            if !self.matches(seg, depth) {
                return false;
            }
            if idx + 1 == path.len() {
                return self.fully_resolves_to(seg, depth);
            }
            if !self.propagate_to_children(seg, depth) {
                return false;
            }
            depth += self.increment_depth_by(seg, depth);
        }
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn key_path_exact_wildcard_and_globstar() {
        let kp = AnimaXKeyPath::from_slices(&["Layer", "Group", "Fill"]);
        assert!(kp.matches_full_path(&["Layer", "Group", "Fill"]));
        assert!(!kp.matches_full_path(&["Layer", "Fill"]));
        assert_eq!(kp.description(), "Layer,Group,Fill");

        let wc = AnimaXKeyPath::from_slices(&["Layer", "*", "Fill"]);
        assert!(wc.matches_full_path(&["Layer", "AnyGroup", "Fill"]));

        let gs = AnimaXKeyPath::from_slices(&["**", "Fill"]);
        assert!(gs.matches_full_path(&["Layer", "Group", "SubGroup", "Fill"]));
        assert!(gs.matches_full_path(&["Fill"]));
        assert!(!gs.matches_full_path(&["Layer", "Stroke"]));

        let all = AnimaXKeyPath::from_slices(&["**"]);
        assert!(all.matches_full_path(&["A", "B", "C"]));
    }

    #[test]
    fn container_transparent_matching() {
        let kp = AnimaXKeyPath::from_slices(&["Layer", "Fill"]);
        assert!(kp.matches(AnimaXKeyPath::CONTAINER_KEY, 0));
        assert_eq!(kp.increment_depth_by(AnimaXKeyPath::CONTAINER_KEY, 0), 0);
        assert!(kp.propagate_to_children(AnimaXKeyPath::CONTAINER_KEY, 1));
    }
}
