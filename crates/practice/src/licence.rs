//! The licences a practice pack's picture may come under: only ones that let the app's users
//! copy, show and share the picture, with at most an attribution (no share-alike, no
//! non-commercial clause). A licence's name is how the source lists write it; its code is how a
//! pack stores it, for good.

/// The terms a picture of a pack is used under.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Licence {
    /// Creative Commons Zero: dedicated to the public domain (the museums' open access).
    Cc0,
    /// A work of the United States federal government (NASA, the Fish and Wildlife Service, the
    /// National Park Service), public domain in the United States.
    PdUsGov,
    /// Public domain by age: its author died long enough ago everywhere it matters.
    PdOld,
    /// Creative Commons Attribution 4.0.
    CcBy4,
    /// Creative Commons Attribution 3.0.
    CcBy3,
    /// Creative Commons Attribution 2.5.
    CcBy25,
    /// Creative Commons Attribution 2.0.
    CcBy2,
}

impl Licence {
    pub const ALL: [Self; 7] = [
        Self::Cc0,
        Self::PdUsGov,
        Self::PdOld,
        Self::CcBy4,
        Self::CcBy3,
        Self::CcBy25,
        Self::CcBy2,
    ];

    /// As stored in a pack; a code names one licence for good.
    #[must_use]
    pub fn code(self) -> u8 {
        match self {
            Self::Cc0 => 1,
            Self::PdUsGov => 2,
            Self::PdOld => 3,
            Self::CcBy4 => 4,
            Self::CcBy3 => 5,
            Self::CcBy25 => 6,
            Self::CcBy2 => 7,
        }
    }

    #[must_use]
    pub fn of_code(code: u8) -> Option<Self> {
        Self::ALL.into_iter().find(|licence| licence.code() == code)
    }

    /// Its short name, as a source list and the app write it.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Cc0 => "CC0",
            Self::PdUsGov => "PD-US-gov",
            Self::PdOld => "PD-old",
            Self::CcBy4 => "CC BY 4.0",
            Self::CcBy3 => "CC BY 3.0",
            Self::CcBy25 => "CC BY 2.5",
            Self::CcBy2 => "CC BY 2.0",
        }
    }

    #[must_use]
    pub fn of_name(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|licence| licence.name() == name)
    }

    /// Whether whoever shows the picture must credit its creator (the Attribution licences).
    #[must_use]
    pub const fn requires_attribution(self) -> bool {
        matches!(self, Self::CcBy4 | Self::CcBy3 | Self::CcBy25 | Self::CcBy2)
    }

    /// The page stating the licence's terms; none for the public domain by law or age.
    #[must_use]
    pub const fn deed(self) -> Option<&'static str> {
        match self {
            Self::Cc0 => Some("https://creativecommons.org/publicdomain/zero/1.0/"),
            Self::PdUsGov | Self::PdOld => None,
            Self::CcBy4 => Some("https://creativecommons.org/licenses/by/4.0/"),
            Self::CcBy3 => Some("https://creativecommons.org/licenses/by/3.0/"),
            Self::CcBy25 => Some("https://creativecommons.org/licenses/by/2.5/"),
            Self::CcBy2 => Some("https://creativecommons.org/licenses/by/2.0/"),
        }
    }
}
