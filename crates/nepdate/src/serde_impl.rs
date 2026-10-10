//! Feature `serde`: a `NepaliDate` is the string `"YYYY-MM-DD"` (ADR-0005, D-11).

use core::fmt;

use serde::de::{self, Deserializer, Visitor};
use serde::{Deserialize, Serialize, Serializer};

use crate::{DateOrder, Lang, NepaliDate, Separator};

#[cfg_attr(docsrs, doc(cfg(feature = "serde")))]
impl Serialize for NepaliDate {
    // `collect_str` writes through `Display`, so no string is built on our side.
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.collect_str(&self.short(DateOrder::Ymd, Separator::Dash, true, Lang::En))
    }
}

#[cfg_attr(docsrs, doc(cfg(feature = "serde")))]
impl<'de> Deserialize<'de> for NepaliDate {
    /// Strict parsing (PARSING §1), the same as [`str::parse`].
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        deserializer.deserialize_str(DateVisitor)
    }
}

struct DateVisitor;

impl Visitor<'_> for DateVisitor {
    type Value = NepaliDate;

    fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("a BS date string \"YYYY-MM-DD\"")
    }

    fn visit_str<E: de::Error>(self, text: &str) -> Result<NepaliDate, E> {
        text.parse().map_err(E::custom)
    }
}
