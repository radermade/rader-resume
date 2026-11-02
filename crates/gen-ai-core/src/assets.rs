//! The workbench's small browser assets, embedded and content-addressed.
//!
//! This table holds only the hand-written entry scripts (`workbench.js`,
//! `nav.js`), a few KB each, served from the Lambda. The multi-megabyte
//! Pyodide runtime and the Python wheels are not here: they are binary and
//! large, so embedding them would bloat the binary and slow cold start.
//! They are vendored into the lab's asset bucket and served under
//! `/gen-ai/assets/pyodide/` behind the same CloudFront behavior, which keeps
//! them on this origin so the workbench policy stays `'self'`. See
//! `phase2/self-hosting.md` for the vendor step.
//!
//! Content-hashed URLs so the cache can be immutable: the filename changes
//! when the bytes change, so a long TTL never serves a stale asset. The
//! renderer asks for the URL and the handler serves the bytes, both from this
//! one table, so the two cannot disagree about a name.

/// One embedded asset.
pub struct Asset {
    /// Stable logical name, used to look the asset up.
    pub name: &'static str,
    pub content_type: &'static str,
    pub body: &'static str,
}

impl Asset {
    /// The content-hashed path this asset is served at.
    ///
    /// FNV-1a over the body, the same non-cryptographic fingerprint the rest
    /// of the project uses for cache keys. It only has to change when the
    /// bytes do.
    pub fn url(&self) -> String {
        let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
        for byte in self.body.as_bytes() {
            hash ^= u64::from(*byte);
            hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
        }

        let (stem, ext) = self.name.rsplit_once('.').unwrap_or((self.name, ""));
        format!("/gen-ai/assets/{stem}.{hash:016x}.{ext}")
    }
}

/// The workbench entry module. Loads Pyodide, mounts the editor, installs the
/// intercept.
pub const WORKBENCH: Asset = Asset {
    name: "workbench.js",
    content_type: "text/javascript; charset=utf-8",
    body: include_str!("../../../assets/gen-ai/workbench.js"),
};

/// The Pyodide worker. Runs in its own thread, loaded from this origin (not a
/// Blob), so `worker-src 'self'` covers it with no `blob:`. A standalone file
/// rather than a string inside `workbench.js`: the embedded-string form was a
/// recurring source of escaping bugs, and a real file has none of that.
pub const WORKER: Asset = Asset {
    name: "worker.js",
    content_type: "text/javascript; charset=utf-8",
    body: include_str!("../../../assets/gen-ai/worker.js"),
};

/// The Previous/Next swap. Enhances the nav form to replace only the lesson
/// region, leaving the workbench mounted. Loaded only on call steps: an
/// overview-to-call transition crosses the policy boundary and must be a full
/// navigation, and the overview's style-only policy forbids script anyway.
pub const NAV: Asset = Asset {
    name: "nav.js",
    content_type: "text/javascript; charset=utf-8",
    body: include_str!("../../../assets/gen-ai/nav.js"),
};

/// Every asset, for the handler to serve.
pub const ALL: &[&Asset] = &[&WORKBENCH, &WORKER, &NAV];

/// Finds an asset by its content-hashed path.
///
/// The handler matches the whole request path against each asset's current
/// URL rather than parsing the hash, so a request for an old hash is a clean
/// miss rather than a stale serve.
pub fn by_url(path: &str) -> Option<&'static Asset> {
    ALL.iter().copied().find(|asset| asset.url() == path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn urls_are_content_addressed_and_stable() {
        let a = WORKBENCH.url();
        assert!(a.starts_with("/gen-ai/assets/workbench."));
        assert!(a.ends_with(".js"));
        assert_eq!(a, WORKBENCH.url(), "same bytes, same url");
    }

    #[test]
    fn a_known_url_resolves_and_an_old_one_misses() {
        let url = WORKBENCH.url();
        assert!(by_url(&url).is_some());
        // A plausible but wrong hash is a miss, not a stale hit.
        assert!(by_url("/gen-ai/assets/workbench.0000000000000000.js").is_none());
    }

    #[test]
    fn the_workbench_asset_is_not_empty() {
        assert!(!WORKBENCH.body.trim().is_empty());
    }
}
