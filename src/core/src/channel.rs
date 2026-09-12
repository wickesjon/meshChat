//! Fixed v1 channel names and presentation IDs; no subscription mutations.
use sha2::{Digest, Sha256};
use unicode_normalization::UnicodeNormalization;

pub const DESCRIPTORS: [&str; 20] = [
    "melodic",
    "hard",
    "deep",
    "dark",
    "dirty",
    "dreamy",
    "hypnotic",
    "euphoric",
    "groovy",
    "filthy",
    "minimal",
    "heavy",
    "bouncy",
    "spacey",
    "gritty",
    "smooth",
    "wonky",
    "lush",
    "uplifting",
    "acid",
];
pub const GENRES: [&str; 20] = [
    "techno",
    "riddim",
    "dubstep",
    "house",
    "dnb",
    "trance",
    "hardstyle",
    "garage",
    "psytrance",
    "jungle",
    "breaks",
    "hardcore",
    "electro",
    "disco",
    "ambient",
    "trap",
    "footwork",
    "gabber",
    "bass",
    "downtempo",
];
pub const LOCATIONS: [&str; 20] = [
    "valley",
    "cave",
    "beach",
    "forest",
    "desert",
    "rooftop",
    "bunker",
    "lagoon",
    "pier",
    "warehouse",
    "canyon",
    "oasis",
    "meadow",
    "glacier",
    "volcano",
    "swamp",
    "grotto",
    "summit",
    "dunes",
    "tundra",
];
pub const PRIVATE_GLYPHS: [&str; 9] = [
    "moon",
    "sun",
    "star",
    "mushroom",
    "crystal",
    "spiral",
    "comet",
    "cactus",
    "disco-ball",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum Error {
    #[error("invalid channel name")]
    Invalid,
    #[error("unknown channel words; update needed")]
    UpdateNeeded,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Public {
    General,
    Confessions,
    EventUpdates,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Private {
    indices: [usize; 3],
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Channel {
    Public(Public),
    Private(Private),
}

/// NFC, trim, lowercase, collapse Unicode whitespace, in that order. Only fixed
/// public names or positional words are accepted by `parse_name` afterward.
pub fn normalize_name(raw: &str) -> Result<String, Error> {
    if raw.is_empty() || raw.len() > 128 {
        return Err(Error::Invalid);
    }
    let nfc: String = raw.nfc().collect();
    Ok(nfc
        .trim()
        .to_lowercase()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" "))
}
pub fn parse_name(raw: &str) -> Result<Channel, Error> {
    let name = normalize_name(raw)?;
    match name.as_str() {
        "#general" => Ok(Channel::Public(Public::General)),
        "#confessions" => Ok(Channel::Public(Public::Confessions)),
        "#event updates" => Ok(Channel::Public(Public::EventUpdates)),
        _ => {
            let mut parts = name.split('|');
            let words = [
                parts.next().ok_or(Error::Invalid)?,
                parts.next().ok_or(Error::Invalid)?,
                parts.next().ok_or(Error::Invalid)?,
            ];
            if parts.next().is_some() {
                return Err(Error::Invalid);
            }
            private(words)
        }
    }
}
pub fn private(words: [&str; 3]) -> Result<Channel, Error> {
    let mut indices = [0; 3];
    for (i, list) in [DESCRIPTORS, GENRES, LOCATIONS].iter().enumerate() {
        if words[i].is_empty()
            || words[i].len() > 9
            || !words[i].bytes().all(|b| b.is_ascii_alphabetic())
        {
            return Err(Error::Invalid);
        }
        indices[i] = list
            .iter()
            .position(|word| word.eq_ignore_ascii_case(words[i]))
            .ok_or(Error::UpdateNeeded)?;
    }
    Ok(Channel::Private(Private { indices }))
}
impl Channel {
    pub fn canonical_name(self) -> String {
        match self {
            Self::Public(Public::General) => "#general".into(),
            Self::Public(Public::Confessions) => "#confessions".into(),
            Self::Public(Public::EventUpdates) => "#event updates".into(),
            Self::Private(p) => format!(
                "{}|{}|{}",
                DESCRIPTORS[p.indices[0]], GENRES[p.indices[1]], LOCATIONS[p.indices[2]]
            ),
        }
    }
    pub fn id(self) -> [u8; 4] {
        let mut hash = Sha256::new();
        hash.update(b"meshfest-v1|");
        hash.update(self.canonical_name());
        let digest = hash.finalize();
        [digest[0], digest[1], digest[2], digest[3]]
    }
    pub fn glyph(self) -> &'static str {
        match self {
            Self::Public(Public::General) => "wave",
            Self::Public(Public::Confessions) => "fire",
            Self::Public(Public::EventUpdates) => "lightning-bolt",
            // The already-hashed channel ID is an unsigned big-endian integer.
            Self::Private(_) => PRIVATE_GLYPHS[(u32::from_be_bytes(self.id()) % 9) as usize],
        }
    }
    /// Only private channels have the specified three-word join route.
    pub fn join_link(self, https: bool) -> Result<String, Error> {
        if !matches!(self, Self::Private(_)) {
            return Err(Error::Invalid);
        }
        Ok(format!(
            "{}{}",
            if https {
                "https://meshfest.app/j/"
            } else {
                "meshfest://j/"
            },
            self.canonical_name().replace('|', "-")
        ))
    }
}
