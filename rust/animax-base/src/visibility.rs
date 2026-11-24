// Copyright 2026 The Lynx Authors. All rights reserved.
// Licensed under the Apache License Version 2.0 that can be found in the
// LICENSE file in the root directory of this source tree.

use serde::{Deserialize, Serialize};

/// Visibility state flags matching `lynx::animax::VisibilityState`.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum VisibilityState {
    Background = 1,
    Visible = 1 << 1,
    Opacity = 1 << 2,
    Size = 1 << 3,
    Attach = 1 << 4,
    Aggregated = 1 << 5,
}

pub const ALL_VISIBILITY_STATES: [VisibilityState; 6] = [
    VisibilityState::Background,
    VisibilityState::Visible,
    VisibilityState::Opacity,
    VisibilityState::Size,
    VisibilityState::Attach,
    VisibilityState::Aggregated,
];

impl VisibilityState {
    pub fn from_u8(v: u8) -> Option<Self> {
        match v {
            1 => Some(Self::Background),
            2 => Some(Self::Visible),
            4 => Some(Self::Opacity),
            8 => Some(Self::Size),
            16 => Some(Self::Attach),
            32 => Some(Self::Aggregated),
            _ => None,
        }
    }
}

pub fn stringify_visibility_state(state: VisibilityState) -> &'static str {
    match state {
        VisibilityState::Background => "kBackground",
        VisibilityState::Visible => "kVisible",
        VisibilityState::Opacity => "kOpacity",
        VisibilityState::Size => "kSize",
        VisibilityState::Attach => "kAttach",
        VisibilityState::Aggregated => "kAggregated",
    }
}

pub fn stringify_visibility_state_flag(state_flag: u8) -> String {
    let mut parts = Vec::new();
    for state in ALL_VISIBILITY_STATES {
        if (state_flag & (state as u8)) != 0 {
            parts.push(stringify_visibility_state(state));
        }
    }
    if parts.is_empty() {
        "None".to_string()
    } else {
        parts.join("|")
    }
}

/// Tracks hidden state bits; the player is visible iff `hidden_flags == 0`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct VisibilityStateFlags {
    hidden_flags: u8,
}

impl VisibilityStateFlags {
    pub fn new() -> Self {
        Self { hidden_flags: 0 }
    }

    pub fn on_hide(&mut self, state: VisibilityState) {
        self.hidden_flags |= state as u8;
    }

    pub fn on_show(&mut self, state: VisibilityState) {
        self.hidden_flags &= !(state as u8);
    }

    pub fn set_flags(&mut self, flags: u8) {
        self.hidden_flags = flags;
    }

    pub fn flags(&self) -> u8 {
        self.hidden_flags
    }

    pub fn is_visible(&self) -> bool {
        self.hidden_flags == 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_visibility_stringify() {
        assert_eq!(stringify_visibility_state_flag(0), "None");
        assert_eq!(
            stringify_visibility_state_flag(
                (VisibilityState::Background as u8) | (VisibilityState::Attach as u8)
            ),
            "kBackground|kAttach"
        );
    }

    #[test]
    fn test_visibility_flags() {
        let mut f = VisibilityStateFlags::new();
        assert!(f.is_visible());
        f.on_hide(VisibilityState::Background);
        assert!(!f.is_visible());
        f.on_hide(VisibilityState::Size);
        f.on_show(VisibilityState::Background);
        assert!(!f.is_visible());
        f.on_show(VisibilityState::Size);
        assert!(f.is_visible());
    }
}
