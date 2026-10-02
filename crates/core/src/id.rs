//! Layer and stroke ids, and the counters that hand them out.

/// Largest id counter an artwork accepts; far beyond any real artwork, so counting never overflows.
pub const MAX_ID: u64 = 1 << 53;

/// Identifies a layer for the artwork's whole life; never reused after deletion.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct LayerId(u64);

impl LayerId {
    /// Wraps a raw id. An id that names no layer is simply unknown to every command.
    #[must_use]
    pub const fn new(raw: u64) -> Self {
        Self(raw)
    }

    #[must_use]
    pub const fn get(self) -> u64 {
        self.0
    }
}

/// Identifies a finished stroke within its artwork; never reused.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct StrokeId(u64);

impl StrokeId {
    #[must_use]
    pub const fn new(raw: u64) -> Self {
        Self(raw)
    }

    #[must_use]
    pub const fn get(self) -> u64 {
        self.0
    }
}

/// The next unused id of each kind. Ids start at 1.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct IdCounters {
    pub(crate) next_layer: u64,
    pub(crate) next_stroke: u64,
}

impl IdCounters {
    pub(crate) fn take_layer(&mut self) -> LayerId {
        let id = LayerId(self.next_layer);
        self.next_layer += 1;
        id
    }

    pub(crate) fn take_stroke(&mut self) -> StrokeId {
        let id = StrokeId(self.next_stroke);
        self.next_stroke += 1;
        id
    }
}
