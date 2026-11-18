//! Shared checks for the fuzz targets: every public reader of a loaded pack must
//! return a value or an error, never panic.

use glibc_provider_sources::{Sources, TABLES};

/// Reads every release table and file through the public API.
pub fn exercise(sources: &Sources) {
    for (name, bytes) in sources.entries() {
        assert!(!name.is_empty());
        if let Some(digest) = name.strip_prefix("blobs/") {
            assert_eq!(sources.blob(digest), Some(bytes));
        }
    }
    for name in sources.releases() {
        let release = sources.release(name).expect("listed release");
        for table in TABLES {
            let _ = release.table(table);
        }
        for key in ["archive", "archive_sha256", "builder.gcc", "builder.pie"] {
            // A loaded pack has passed validation, which requires these keys.
            assert!(release.value(key).is_some(), "{name}: {key}");
        }
        // Validation already parsed these; they must parse again.
        for file in release.files().expect("validated files.tsv") {
            if let Some(size) = file.size {
                assert_eq!(
                    sources.blob(file.sha256).map(|b| b.len() as u64),
                    Some(size)
                );
            }
        }
        for header in release.headers().expect("validated headers.tsv") {
            assert_eq!(
                sources.blob(header.sha256).map(|b| b.len() as u64),
                Some(header.size)
            );
        }
        release.objects().expect("validated objects.tsv");
    }
}
