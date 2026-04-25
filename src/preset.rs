use std::str::FromStr;

use crate::reader::ChunkKind;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Preset {
    Gentle,
    Standard,
    Technical,
    Study,
}

impl Preset {
    pub fn cycle(self) -> Self {
        match self {
            Preset::Gentle => Preset::Standard,
            Preset::Standard => Preset::Technical,
            Preset::Technical => Preset::Study,
            Preset::Study => Preset::Gentle,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Preset::Gentle => "gentle",
            Preset::Standard => "standard",
            Preset::Technical => "technical",
            Preset::Study => "study",
        }
    }

    pub fn default_wpm(self) -> u32 {
        match self {
            Preset::Gentle => 250,
            Preset::Standard => 300,
            Preset::Technical => 240,
            Preset::Study => 220,
        }
    }

    pub fn chunk_multiplier(self, kind: ChunkKind) -> f32 {
        match self {
            Preset::Gentle => match kind {
                ChunkKind::Heading => 1.15,
                ChunkKind::Code | ChunkKind::Table => 1.15,
                ChunkKind::Paragraph => 1.25,
                _ => 1.08,
            },
            Preset::Standard => 1.0,
            Preset::Technical => match kind {
                ChunkKind::Heading => 1.1,
                ChunkKind::Code | ChunkKind::Table => 1.35,
                ChunkKind::Paragraph => 1.2,
                _ => 1.0,
            },
            Preset::Study => match kind {
                ChunkKind::Heading => 1.2,
                ChunkKind::Code | ChunkKind::Table => 1.25,
                ChunkKind::Paragraph => 1.35,
                _ => 1.12,
            },
        }
    }
}

impl FromStr for Preset {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "gentle" => Ok(Preset::Gentle),
            "standard" => Ok(Preset::Standard),
            "technical" => Ok(Preset::Technical),
            "study" => Ok(Preset::Study),
            _ => Err(()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_known_presets() {
        assert_eq!("gentle".parse::<Preset>(), Ok(Preset::Gentle));
        assert_eq!("standard".parse::<Preset>(), Ok(Preset::Standard));
        assert_eq!("technical".parse::<Preset>(), Ok(Preset::Technical));
        assert_eq!("study".parse::<Preset>(), Ok(Preset::Study));
        assert!("unknown".parse::<Preset>().is_err());
    }
}
