# W3-00c: the tithi `*` marker

The trailing `*` in some Nepali tithi cells is part of the tithi name in the data, not something
the widget adds. It comes from the hub JSON through `cargo xtask codegen`:

- `shared/data/patro/2078.json`, day `2078-02-08`: `"np": "एकादशी*", "en": "Ekadashi (K)"`.
- `crates/nepdate-patro/src/generated/patro.rs`: `TITHI_NAMES` holds the same pair, and
  `Tithi::name(Lang::Ne)` returns it unchanged.
- The upstream C# data has the same strings in
  `src/NepDate/Core/Dictionaries/CalendarStrings.cs` (`("एकादशी*", "Ekadashi (K)")`), with no
  comment on the marker.

Across the 1,468 days in `shared/data/patro/2080` to `2084`, 69 days (4.7 %) carry a starred
name. Over the whole data set, 2077 to 2084, 152 of the 2,190 days that have a tithi are starred
(2078 to 2081 only), with 16 distinct starred names. The English form spells the marker out as
`(K)`. Starred and unstarred names both occur in the same lunar month: for example, 2078-01-31
is `तृतिया*` and 2078-02-01 is `तृतिया`.

Meaning: `shared/spec/PATRO.md` defines a tithi as an `{np, en}` pair and does not define `*` or
`(K)`. `shared/data/README.md` says the metadata came from an upstream scraper whose source is
not published, and lists that provenance as an open data question. `K` reads like *kshaya* (a
tithi that never touches a sunrise and is skipped), but the data does not support that reading
(W3 review): compared with the day before, 117 of the 152 starred days are simply the next tithi,
22 come after a skipped tithi and 3 repeat it, while 56 skips and 49 repeats carry no star. No
file in this repository or in the C# source defines it. Because neither the specification nor a library document defines the
marker, the widget keeps the name exactly as the library returns it. Removing it would hide
information and invent a rule the data does not state.

Action: report to the hub (the `shared/` copy is read-only here). The widget changes nothing.
