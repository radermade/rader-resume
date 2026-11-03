//! The resume's embedded binary assets, content-addressed.
//!
//! Only the portrait lives here: a couple of small images, embedded and served
//! from the handler at a content-hashed path. The renderer asks for the URL and
//! the handler serves the bytes from this one table, so the two cannot disagree
//! about a name.
//!
//! `resume-core` stays free of AWS and lambda types: this is plain bytes and a
//! hash, no I/O, so it compiles anywhere the crate does.

/// One embedded binary asset.
pub struct BinaryAsset {
    /// Stable logical name, used to derive the served path.
    pub name: &'static str,
    pub content_type: &'static str,
    pub bytes: &'static [u8],
}

impl BinaryAsset {
    /// The content-hashed path this asset is served at.
    ///
    /// FNV-1a over the bytes, the same non-cryptographic fingerprint the rest
    /// of the project uses for cache keys. The filename changes when the bytes
    /// change, so a long immutable TTL never serves a stale asset.
    pub fn url(&self) -> String {
        let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
        for byte in self.bytes {
            hash ^= u64::from(*byte);
            hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
        }

        let (stem, ext) = self.name.rsplit_once('.').unwrap_or((self.name, ""));
        format!("/assets/{stem}.{hash:016x}.{ext}")
    }
}

/// The portrait, modern format. Served to anything that accepts WebP, which is
/// every current browser; a few KB after resizing to the hex's display size.
pub const PORTRAIT_WEBP: BinaryAsset = BinaryAsset {
    name: "portrait.webp",
    content_type: "image/webp",
    bytes: include_bytes!("../../../assets/resume/portrait.webp"),
};

/// The portrait fallback, PNG, for anything that cannot take the WebP. Larger,
/// but it is the fallback leg of a `<picture>`, so a modern browser never
/// fetches it.
pub const PORTRAIT_PNG: BinaryAsset = BinaryAsset {
    name: "portrait.png",
    content_type: "image/png",
    bytes: include_bytes!("../../../assets/resume/portrait.png"),
};

/// The portrait's intrinsic display dimensions, for explicit width/height on
/// the element so the layout does not shift as the image loads. The source is
/// resized to 200px wide; the rendered box is smaller, but the ratio is what
/// matters for reserving space.
pub const PORTRAIT_WIDTH: u32 = 200;
pub const PORTRAIT_HEIGHT: u32 = 174;

/// Every binary asset, for the handler to serve by URL.
pub const ALL: &[&BinaryAsset] = &[&PORTRAIT_WEBP, &PORTRAIT_PNG];

/// Finds an embedded asset by its content-hashed path, or `None`.
///
/// Compares against the computed URL rather than parsing the hash out, so a
/// request for an old hash is a clean miss rather than a stale serve.
pub fn by_url(path: &str) -> Option<&'static BinaryAsset> {
    ALL.iter().copied().find(|asset| asset.url() == path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_portrait_assets_are_present_and_nonempty() {
        assert!(!PORTRAIT_WEBP.bytes.is_empty());
        assert!(!PORTRAIT_PNG.bytes.is_empty());
    }

    #[test]
    fn a_url_round_trips_through_by_url() {
        let url = PORTRAIT_WEBP.url();
        let found = by_url(&url).expect("the webp resolves from its own url");
        assert_eq!(found.name, PORTRAIT_WEBP.name);
        // A wrong hash misses cleanly.
        assert!(by_url("/assets/portrait.0000000000000000.webp").is_none());
    }

    #[test]
    fn the_webp_is_the_modern_primary_and_stays_small() {
        // The whole point of shipping WebP: a modern browser fetches only this
        // leg, so it must be the small one. A regression that let the PNG grow
        // into the primary slot would defeat the page-weight budget.
        assert!(
            PORTRAIT_WEBP.bytes.len() < PORTRAIT_PNG.bytes.len(),
            "the webp must be smaller than the png fallback"
        );
    }
}
