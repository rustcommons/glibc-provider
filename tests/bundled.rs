use glibc_provider_sources::{Sources, RELEASES};
use std::fs;

#[test]
fn bundled_pack_holds_every_release_with_its_inputs() {
    let sources = Sources::bundled().unwrap();
    assert_eq!(sources.releases(), RELEASES);
    for &name in RELEASES {
        let release = sources.release(name).unwrap();
        assert_eq!(release.value("release"), Some(name));
        assert_eq!(
            release.value("archive"),
            Some(format!("glibc-{name}.tar.xz").as_str())
        );
        let objects = release.objects().unwrap();
        for target in ["crt1.o", "crti.o", "crtn.o", "libc_nonshared.a"] {
            assert!(
                objects.iter().any(|o| o.target == target),
                "{name} {target}"
            );
        }
        // libpthread_nonshared.a merged into libc_nonshared.a in 2.28.
        let pthread = objects.iter().any(|o| o.target == "libpthread_nonshared.a");
        assert_eq!(
            pthread,
            release.value("target.libpthread_nonshared.a").is_some()
        );
        let headers = release.headers().unwrap();
        assert!(headers.iter().any(|h| h.path == "features.h"), "{name}");
        assert!(
            headers.iter().any(|h| h.path.starts_with("linux/")),
            "{name}"
        );
        let files = release.files().unwrap();
        assert!(files.iter().any(|f| f.class == "builder"), "{name}");
        // Every object's interface is recorded, with one allocated-bytes digest.
        let mut distinct: Vec<&str> = objects.iter().map(|o| o.object).collect();
        distinct.sort();
        distinct.dedup();
        let interfaces = release.table("interfaces.tsv").unwrap();
        let digests = interfaces
            .lines()
            .filter(|l| l.contains("\tbytes\t"))
            .count();
        assert_eq!(digests, distinct.len(), "{name}");
    }
}

#[test]
fn writes_a_release_tree_with_the_recorded_contents() {
    let sources = Sources::bundled().unwrap();
    let release = sources.release("2.17").unwrap();
    let dir = std::env::temp_dir().join(format!("gps-bundled-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir(&dir).unwrap();
    release.write_sources(&dir.join("inputs")).unwrap();
    release.write_headers(&dir.join("include")).unwrap();
    let start = fs::read_to_string(dir.join("inputs/src/sysdeps/x86_64/start.S")).unwrap();
    assert!(start.contains("_start"));
    assert!(dir.join("inputs/build/config.h").is_file());
    assert!(dir.join("inputs/builder-defaults.txt").is_file());
    for file in release.files().unwrap() {
        let path = match file.class {
            "archive" => format!("inputs/src/{}", file.path),
            "generated" => format!("inputs/build/{}", file.path),
            "uapi" => format!("inputs/uapi/{}", file.path),
            "builder" => format!("inputs/{}", file.path),
            _ => continue,
        };
        assert_eq!(
            fs::read(dir.join(&path)).unwrap(),
            sources.blob(file.sha256).unwrap(),
            "{path}"
        );
    }
    let headers = release.headers().unwrap();
    assert_eq!(
        fs::read(dir.join("include/features.h")).unwrap().len() as u64,
        headers
            .iter()
            .find(|h| h.path == "features.h")
            .unwrap()
            .size
    );
    fs::remove_dir_all(&dir).unwrap();
}
