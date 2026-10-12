//! Writes the glibc-provider-sources pack from `data/`.
//!
//! glibc-provider-pack [--root DIR] [--check]
//!
//! The pack holds every release's tables and every file they name, stored once,
//! each as a line delta against its previous version where that is smaller.
//! Files are ordered by the path and release where each first appears, so that
//! consecutive versions of a file sit together and compress against each other.
//! The pack is compressed with `gzip -9 -n` into `pack/sources.gz`. With --check,
//! the existing pack is decoded with the crate itself and must hold exactly the
//! same entries as `data/`; the compressed bytes are not compared.

use glibc_provider_sources::{encode, PackEntry, Sources, TABLES};
use std::collections::BTreeMap;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

fn rows(path: &Path) -> Result<Vec<Vec<String>>, String> {
    let text =
        fs::read_to_string(path).map_err(|e| format!("cannot read {}: {e}", path.display()))?;
    Ok(text
        .lines()
        .skip(1)
        .map(|l| l.split('\t').map(String::from).collect())
        .collect())
}

/// Every entry with its delta base: a table's base is the same table of the
/// previous release, a file's the previous version of the same path.
fn entries(data: &Path) -> Result<Vec<PackEntry>, String> {
    let mut releases: Vec<String> = fs::read_dir(data)
        .map_err(|e| e.to_string())?
        .filter_map(|e| e.ok()?.file_name().into_string().ok())
        .filter(|name| name.starts_with("2."))
        .collect();
    let key = |r: &String| -> Vec<u32> { r.split('.').map(|p| p.parse().unwrap_or(0)).collect() };
    releases.sort_by_key(key);
    if releases.as_slice() != glibc_provider_sources::RELEASES {
        return Err(format!(
            "data/ releases {releases:?} differ from the crate's RELEASES"
        ));
    }
    let mut out = Vec::new();
    // sha256 -> (path, release index) where the file first appears.
    let mut first: BTreeMap<String, (String, usize)> = BTreeMap::new();
    for table in TABLES {
        for (index, release) in releases.iter().enumerate() {
            let bytes = fs::read(data.join(release).join(table))
                .map_err(|e| format!("{release}/{table}: {e}"))?;
            let base = index
                .checked_sub(1)
                .map(|previous| format!("{}/{table}", releases[previous]));
            out.push((format!("{release}/{table}"), bytes, base));
        }
    }
    for (index, release) in releases.iter().enumerate() {
        let dir = data.join(release);
        for row in rows(&dir.join("files.tsv"))? {
            if row[0] != "compiler" {
                first
                    .entry(row[2].clone())
                    .or_insert((row[1].clone(), index));
            }
        }
        for row in rows(&dir.join("headers.tsv"))? {
            first
                .entry(row[1].clone())
                .or_insert((row[0].clone(), index));
        }
    }
    let mut blobs: Vec<(&String, &(String, usize))> = first.iter().collect();
    blobs.sort_by(|a, b| (&a.1 .0, a.1 .1, a.0).cmp(&(&b.1 .0, b.1 .1, b.0)));
    let mut previous: Option<(&String, &String)> = None;
    for (digest, (first_path, _)) in blobs {
        let path = data.join("blobs").join(&digest[..2]).join(digest);
        let bytes = fs::read(&path).map_err(|e| format!("{}: {e}", path.display()))?;
        let base = previous
            .filter(|(path, _)| *path == first_path)
            .map(|(_, digest)| format!("blobs/{digest}"));
        out.push((format!("blobs/{digest}"), bytes, base));
        previous = Some((first_path, digest));
    }
    Ok(out)
}

fn gzip(data: &[u8]) -> Result<Vec<u8>, String> {
    let mut child = Command::new("gzip")
        .args(["-9", "-n", "-c"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .map_err(|e| format!("cannot run gzip: {e}"))?;
    let mut stdin = child.stdin.take().expect("piped");
    let input = data.to_vec();
    let writer = std::thread::spawn(move || stdin.write_all(&input));
    let output = child.wait_with_output().map_err(|e| e.to_string())?;
    writer
        .join()
        .map_err(|_| "gzip writer panicked")?
        .map_err(|e| e.to_string())?;
    if !output.status.success() {
        return Err("gzip failed".into());
    }
    Ok(output.stdout)
}

fn run() -> Result<(), String> {
    let mut root = PathBuf::from(".");
    let mut check = false;
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--root" => root = PathBuf::from(args.next().ok_or("--root needs a value")?),
            "--check" => check = true,
            _ => return Err(format!("unknown argument {arg}")),
        }
    }
    let expected = entries(&root.join("data"))?;
    let pack = root.join("pack/sources.gz");
    if check {
        let bytes = fs::read(&pack).map_err(|e| format!("{}: {e}", pack.display()))?;
        let sources = Sources::from_gzip(&bytes)?;
        let actual: BTreeMap<&str, &[u8]> = sources.entries().collect();
        let wanted: BTreeMap<&str, &[u8]> = expected
            .iter()
            .map(|(n, b, _)| (n.as_str(), b.as_slice()))
            .collect();
        if actual != wanted {
            let differ: Vec<_> = wanted
                .keys()
                .chain(actual.keys())
                .filter(|n| actual.get(*n) != wanted.get(*n))
                .take(5)
                .collect();
            return Err(format!("pack differs from data/, for example {differ:?}"));
        }
        println!(
            "{} matches data/: {} entries, {} bytes compressed",
            pack.display(),
            actual.len(),
            bytes.len()
        );
        return Ok(());
    }
    let raw = encode(&expected)?;
    let compressed = gzip(&raw)?;
    // The written pack must read back through the crate.
    Sources::from_gzip(&compressed)?;
    fs::create_dir_all(pack.parent().expect("has a parent")).map_err(|e| e.to_string())?;
    fs::write(&pack, &compressed).map_err(|e| e.to_string())?;
    println!(
        "wrote {}: {} entries, {} bytes, {} compressed",
        pack.display(),
        expected.len(),
        raw.len(),
        compressed.len()
    );
    Ok(())
}

fn main() {
    if let Err(message) = run() {
        eprintln!("glibc-provider-pack: {message}");
        std::process::exit(1);
    }
}
