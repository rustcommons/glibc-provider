//! The exact glibc sources behind cargo-libc's link providers, for x86_64 Linux
//! and glibc 2.17 through 2.44.
//!
//! A link provider lets Rust build for an older glibc than the host has. Besides
//! link stubs, it needs real pieces of that glibc release: the startup objects
//! (`crt1.o`, `crti.o`, `crtn.o`), `libc_nonshared.a` (and `libpthread_nonshared.a`
//! before 2.28), and the installed headers. For each release this crate holds:
//!
//! - every source file those objects are compiled from, unmodified from the pinned
//!   GNU release archive, with the headers glibc's build generated for them and the
//!   builder's Linux UAPI headers;
//! - the exact compile commands, and the builder compiler's defaults those commands
//!   leave implicit;
//! - the interface of every object the pinned container build produced, which a
//!   local rebuild must reproduce;
//! - the release's installed headers.
//!
//! With it, linux-abi-tools builds a provider on the user's machine with the host's
//! GCC, offline, with no glibc binaries shipped. The data is recorded by the
//! [glibc-provider](https://github.com/rustcommons/glibc-provider) collector from
//! qualified container builds.
//!
//! The data is compiled in as one gzip-compressed pack (3.7 MB). [`Sources::bundled`]
//! decompresses it (66 MiB) and checks every file's SHA-256.
//!
//! ```
//! let sources = glibc_provider_sources::Sources::bundled().unwrap();
//! let release = sources.release("2.28").unwrap();
//! assert_eq!(release.value("archive"), Some("glibc-2.28.tar.xz"));
//! let start = release.objects().unwrap().into_iter().find(|o| o.object == "csu/start.o").unwrap();
//! assert!(start.command.starts_with("gcc ../sysdeps/x86_64/start.S -c"));
//! assert!(release.headers().unwrap().iter().any(|h| h.path == "stdio.h"));
//! ```
//!
//! **Licenses.** The glibc files and generated headers are glibc's, under the GNU
//! LGPL version 2.1 or later; so is this crate's code. The Linux UAPI headers are the
//! kernel's, each under its own SPDX tag, mostly `GPL-2.0 WITH Linux-syscall-note`
//! (the note keeps programs using them outside the GPL). See [`LICENSE`] and
//! `NOTICE.md`.

#![warn(missing_docs)]

mod delta;
mod inflate;
mod sha256;

use std::{
    collections::BTreeMap,
    fs,
    ops::Range,
    path::{Component, Path},
};

/// The first line of an uncompressed pack.
pub const FORMAT: &str = "glibc-provider-pack 1";
/// SPDX license expression of the crate: its code and the glibc files, and the
/// licenses of the Linux UAPI headers (where a header is dual-licensed, the
/// expression names the GPL choice).
pub const LICENSE: &str = "LGPL-2.1-or-later AND GPL-2.0-only WITH Linux-syscall-note AND GPL-2.0-or-later WITH Linux-syscall-note AND LGPL-2.1-only WITH Linux-syscall-note AND LGPL-2.1-or-later WITH Linux-syscall-note AND BSD-3-Clause AND MIT";
/// The glibc releases with data, oldest first.
pub const RELEASES: &[&str] = &[
    "2.17", "2.18", "2.19", "2.20", "2.21", "2.22", "2.23", "2.24", "2.25", "2.26", "2.27", "2.28",
    "2.29", "2.30", "2.31", "2.32", "2.33", "2.34", "2.35", "2.36", "2.37", "2.38", "2.39", "2.40",
    "2.41", "2.42", "2.43", "2.44",
];
/// The tables each release has.
pub const TABLES: [&str; 6] = [
    "release.tsv",
    "objects.tsv",
    "inputs.tsv",
    "files.tsv",
    "headers.tsv",
    "interfaces.tsv",
];
/// Largest decompressed pack accepted.
const MAX_PACK: usize = 512 << 20;

static PACK: &[u8] = include_bytes!("../pack/sources.gz");

/// The decompressed, verified pack: per-release tables and the files they name.
pub struct Sources {
    data: Vec<u8>,
    entries: BTreeMap<String, Range<usize>>,
}

/// One release's tables.
#[derive(Clone, Copy)]
pub struct Release<'a> {
    sources: &'a Sources,
    /// The release, such as `"2.28"`.
    pub name: &'a str,
}

/// A recorded input file (`files.tsv`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SourceFile<'a> {
    /// `archive` (from the GNU archive), `generated` (by glibc's build), `uapi`
    /// (Linux UAPI header), `builder` (the builder compiler's predefined macros) or
    /// `compiler` (the builder compiler's own header, recorded by path only).
    pub class: &'a str,
    /// Relative to the source tree, build tree or include directory of its class;
    /// absolute in the builder for `compiler`.
    pub path: &'a str,
    /// The file's SHA-256 (lowercase hex); empty for `compiler`.
    pub sha256: &'a str,
    /// The file's size in bytes; `None` for `compiler`.
    pub size: Option<u64>,
}

/// An installed header (`headers.tsv`), relative to `usr/include`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Header<'a> {
    /// Relative to `usr/include`, such as `stdio.h` or `linux/errno.h`.
    pub path: &'a str,
    /// The header's SHA-256 (lowercase hex).
    pub sha256: &'a str,
    /// The header's size in bytes.
    pub size: u64,
}

/// One object behind a target (`objects.tsv`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Object<'a> {
    /// `crt1.o`, `crti.o`, `crtn.o`, `libc_nonshared.a` or `libpthread_nonshared.a`.
    pub target: &'a str,
    /// Position in an archive target; 0 otherwise.
    pub member: u32,
    /// Path in the build tree, such as `csu/start.o`.
    pub object: &'a str,
    /// `compile` or `relocatable` (a `-r` link of other objects).
    pub kind: &'a str,
    /// The source subdirectory the command runs in.
    pub directory: &'a str,
    /// The recorded command; `@SRC@` is the source tree and `@BUILD@` the build tree.
    pub command: &'a str,
}

fn rows<'a>(text: &'a str, header: &str) -> Result<Vec<Vec<&'a str>>, String> {
    let mut lines = text.lines();
    if lines.next() != Some(header) {
        return Err(format!("expected header {header:?}"));
    }
    let width = header.split('\t').count();
    lines
        .map(|line| {
            let fields: Vec<&str> = line.split('\t').collect();
            if fields.len() == width {
                Ok(fields)
            } else {
                Err(format!("malformed row {line:?}"))
            }
        })
        .collect()
}

fn safe(path: &str) -> bool {
    !path.is_empty()
        && !path.contains('\\')
        && Path::new(path)
            .components()
            .all(|c| matches!(c, Component::Normal(_)))
}

fn hex64(text: &str) -> bool {
    text.len() == 64 && text.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f'))
}

/// Most differing lines searched for a delta; beyond it an entry is stored whole.
const DELTA_LIMIT: usize = 3000;

/// A pack entry to write: name, contents, and the earlier entry it may be a delta of.
pub type PackEntry = (String, Vec<u8>, Option<String>);

/// Writes an uncompressed pack of `entries`, in the given order. An entry with a
/// base, which must come earlier, is stored as a line delta against it when that
/// is smaller.
pub fn encode(entries: &[PackEntry]) -> Result<Vec<u8>, String> {
    let mut position: BTreeMap<&str, usize> = BTreeMap::new();
    let mut index = format!("{FORMAT}\n{}\n", entries.len());
    let mut bodies = Vec::new();
    for (number, (name, contents, base)) in entries.iter().enumerate() {
        let mut stored = (contents.clone(), "-");
        if let Some(base) = base {
            let &at = position
                .get(base.as_str())
                .ok_or_else(|| format!("{name}: base {base} must come earlier"))?;
            let base_contents: &Vec<u8> = &entries[at].1;
            if let Some(delta) = delta::diff(base_contents, contents, DELTA_LIMIT) {
                if delta.len() < contents.len() {
                    stored = (delta, base.as_str());
                }
            }
        }
        index.push_str(&format!(
            "{name}\t{}\t{}\t{}\t{}\n",
            sha256::hex(&sha256::sha256(contents)),
            contents.len(),
            stored.0.len(),
            stored.1
        ));
        bodies.push(stored.0);
        position.insert(name.as_str(), number);
    }
    let mut out = index.into_bytes();
    for body in bodies {
        out.extend_from_slice(&body);
    }
    Ok(out)
}

impl Sources {
    /// The pack compiled into this crate.
    pub fn bundled() -> Result<Self, String> {
        Self::from_gzip(PACK)
    }

    /// A gzip-compressed pack.
    pub fn from_gzip(bytes: &[u8]) -> Result<Self, String> {
        Self::parse(inflate::gunzip(bytes, MAX_PACK)?)
    }

    /// An uncompressed pack. Every entry's SHA-256 is checked, and every file a
    /// release's tables name must be present with that hash and size.
    pub fn parse(data: Vec<u8>) -> Result<Self, String> {
        let mut lines = 0;
        let mut offset = 0;
        let mut line = || -> Result<&str, String> {
            let end = data[offset..]
                .iter()
                .position(|&b| b == b'\n')
                .ok_or("pack index ends early")?;
            let text = std::str::from_utf8(&data[offset..offset + end])
                .map_err(|_| "pack index is not UTF-8")?;
            offset += end + 1;
            lines += 1;
            Ok(text)
        };
        if line()? != FORMAT {
            return Err(format!("not a {FORMAT:?} pack"));
        }
        let count: usize = line()?.parse().map_err(|_| "invalid entry count")?;
        if count > 1_000_000 {
            return Err("too many pack entries".into());
        }
        let mut index = Vec::with_capacity(count);
        for _ in 0..count {
            let text = line()?;
            let [name, digest, size, stored, base] = text.split('\t').collect::<Vec<_>>()[..]
            else {
                return Err(format!("malformed pack entry {text:?}"));
            };
            let size: usize = size.parse().map_err(|_| "invalid entry size")?;
            let stored: usize = stored.parse().map_err(|_| "invalid stored size")?;
            if !safe(name) || !hex64(digest) || (base != "-" && !safe(base)) {
                return Err(format!("invalid pack entry {name:?}"));
            }
            let base = (base != "-").then(|| base.to_string());
            index.push((name.to_string(), digest.to_string(), size, stored, base));
        }
        // Rebuild each entry, from its stored bytes or a delta against an earlier one.
        let mut contents: Vec<u8> = Vec::new();
        let mut entries: BTreeMap<String, Range<usize>> = BTreeMap::new();
        let mut start = offset;
        for (name, digest, size, stored, base) in index {
            let end = start.checked_add(stored).filter(|&e| e <= data.len());
            let end = end.ok_or_else(|| format!("{name}: pack ends early"))?;
            let body = &data[start..end];
            let rebuilt = match &base {
                None => body.to_vec(),
                Some(base) => {
                    let range = entries
                        .get(base)
                        .ok_or_else(|| format!("{name}: base {base} must come earlier"))?;
                    delta::apply(&contents[range.clone()], body)
                        .map_err(|e| format!("{name}: {e}"))?
                }
            };
            if rebuilt.len() != size || sha256::hex(&sha256::sha256(&rebuilt)) != digest {
                return Err(format!("{name}: SHA-256 or size mismatch"));
            }
            if let Some(blob) = name.strip_prefix("blobs/") {
                if blob != digest {
                    return Err(format!("{name}: named by another hash"));
                }
            }
            let at = contents.len();
            contents.extend_from_slice(&rebuilt);
            if entries.insert(name.clone(), at..contents.len()).is_some() {
                return Err(format!("{name}: duplicate pack entry"));
            }
            start = end;
        }
        if start != data.len() {
            return Err("pack has trailing data".into());
        }
        let sources = Self {
            data: contents,
            entries,
        };
        for release in sources.releases() {
            sources.release(release)?.validate()?;
        }
        Ok(sources)
    }

    /// Every entry, by name.
    pub fn entries(&self) -> impl Iterator<Item = (&str, &[u8])> {
        self.entries
            .iter()
            .map(|(name, range)| (name.as_str(), &self.data[range.clone()]))
    }

    /// The releases with tables, oldest first.
    pub fn releases(&self) -> Vec<&str> {
        let mut releases: Vec<&str> = self
            .entries
            .keys()
            .filter_map(|name| name.strip_suffix("/release.tsv"))
            .collect();
        releases.sort_by_key(|r| {
            r.split('.')
                .map(|p| p.parse::<u32>().unwrap_or(u32::MAX))
                .collect::<Vec<_>>()
        });
        releases
    }

    /// The tables of one release, such as `"2.28"`.
    pub fn release(&self, release: &str) -> Result<Release<'_>, String> {
        let name = self
            .entries
            .get_key_value(&format!("{release}/release.tsv"))
            .and_then(|(key, _)| key.strip_suffix("/release.tsv"))
            .ok_or_else(|| format!("no data for glibc {release}"))?;
        Ok(Release {
            sources: self,
            name,
        })
    }

    /// A stored file by its SHA-256.
    pub fn blob(&self, sha256: &str) -> Option<&[u8]> {
        self.entries
            .get(&format!("blobs/{sha256}"))
            .map(|range| &self.data[range.clone()])
    }
}

impl<'a> Release<'a> {
    /// One of [`TABLES`], as text.
    pub fn table(&self, table: &str) -> Result<&'a str, String> {
        let range = self
            .sources
            .entries
            .get(&format!("{}/{table}", self.name))
            .ok_or_else(|| format!("glibc {}: no {table}", self.name))?;
        std::str::from_utf8(&self.sources.data[range.clone()])
            .map_err(|_| format!("glibc {}: {table} is not UTF-8", self.name))
    }

    /// A `release.tsv` value: `archive`, `archive_sha256`, `builder.gcc`,
    /// `builder.pie`, `target.crt1.o` and so on.
    pub fn value(&self, key: &str) -> Option<&'a str> {
        let rows = rows(self.table("release.tsv").ok()?, "key\tvalue").ok()?;
        rows.into_iter().find(|r| r[0] == key).map(|r| r[1])
    }

    /// The recorded inputs of the release's objects (`files.tsv`).
    pub fn files(&self) -> Result<Vec<SourceFile<'a>>, String> {
        rows(self.table("files.tsv")?, "class\tpath\tsha256\tsize")?
            .into_iter()
            .map(|r| {
                let size = if r[0] == "compiler" {
                    None
                } else {
                    Some(
                        r[3].parse()
                            .map_err(|_| format!("{}: invalid size", r[1]))?,
                    )
                };
                Ok(SourceFile {
                    class: r[0],
                    path: r[1],
                    sha256: r[2],
                    size,
                })
            })
            .collect()
    }

    /// The release's installed headers (`headers.tsv`).
    pub fn headers(&self) -> Result<Vec<Header<'a>>, String> {
        rows(self.table("headers.tsv")?, "path\tsha256\tsize")?
            .into_iter()
            .map(|r| {
                Ok(Header {
                    path: r[0],
                    sha256: r[1],
                    size: r[2]
                        .parse()
                        .map_err(|_| format!("{}: invalid size", r[0]))?,
                })
            })
            .collect()
    }

    /// Every object behind the release's targets, with its command (`objects.tsv`).
    pub fn objects(&self) -> Result<Vec<Object<'a>>, String> {
        rows(
            self.table("objects.tsv")?,
            "target\tmember\tobject\tkind\tdirectory\tcommand",
        )?
        .into_iter()
        .map(|r| {
            Ok(Object {
                target: r[0],
                member: r[1]
                    .parse()
                    .map_err(|_| format!("{}: invalid member", r[2]))?,
                object: r[2],
                kind: r[3],
                directory: r[4],
                command: r[5],
            })
        })
        .collect()
    }

    fn validate(&self) -> Result<(), String> {
        for table in TABLES {
            self.table(table)?;
        }
        for key in ["archive", "archive_sha256", "builder.gcc", "builder.pie"] {
            self.value(key)
                .ok_or_else(|| format!("glibc {}: release.tsv lacks {key}", self.name))?;
        }
        let check = |path: &str, digest: &str, size: u64| -> Result<(), String> {
            let blob = self
                .sources
                .blob(digest)
                .ok_or_else(|| format!("glibc {}: {path}: no stored file", self.name))?;
            if blob.len() as u64 != size || !safe(path) {
                return Err(format!("glibc {}: {path}: wrong size or path", self.name));
            }
            Ok(())
        };
        for file in self.files()? {
            match (file.class, file.size) {
                ("compiler", _) => {}
                ("archive" | "generated" | "uapi" | "builder", Some(size)) => {
                    check(file.path, file.sha256, size)?
                }
                (class, _) => return Err(format!("glibc {}: unknown class {class}", self.name)),
            }
        }
        for header in self.headers()? {
            check(header.path, header.sha256, header.size)?;
        }
        for object in self.objects()? {
            if !safe(object.object) || !matches!(object.kind, "compile" | "relocatable") {
                return Err(format!(
                    "glibc {}: invalid object {}",
                    self.name, object.object
                ));
            }
        }
        Ok(())
    }

    fn write(&self, root: &Path, files: &[(&str, &str)]) -> Result<(), String> {
        fs::create_dir(root).map_err(|e| format!("cannot create new {}: {e}", root.display()))?;
        for (path, digest) in files {
            let target = root.join(path);
            if let Some(parent) = target.parent() {
                fs::create_dir_all(parent).map_err(|e| e.to_string())?;
            }
            let blob = self.sources.blob(digest).expect("validated");
            fs::write(&target, blob).map_err(|e| format!("{}: {e}", target.display()))?;
        }
        Ok(())
    }

    /// Writes the inputs into the new directory `dir`: `src/` (archive files),
    /// `build/` (generated headers), `uapi/` (Linux UAPI headers) and
    /// `builder-defaults.txt`. A compile command runs in `src/<directory>`, with
    /// `@SRC@` and `@BUILD@` replaced by `dir/src` and `dir/build`, and `dir/uapi` as
    /// its only system include directory besides the compiler's own.
    pub fn write_sources(&self, dir: &Path) -> Result<(), String> {
        let files = self.files()?;
        let mut placed = Vec::new();
        let mut paths = Vec::new();
        for file in &files {
            let path = match file.class {
                "archive" => format!("src/{}", file.path),
                "generated" => format!("build/{}", file.path),
                "uapi" => format!("uapi/{}", file.path),
                "builder" => file.path.to_string(),
                _ => continue,
            };
            paths.push(path);
            placed.push(file.sha256);
        }
        let pairs: Vec<(&str, &str)> = paths
            .iter()
            .map(String::as_str)
            .zip(placed.iter().copied())
            .collect();
        self.write(dir, &pairs)
    }

    /// Writes the installed headers into the new directory `dir` (a `usr/include`).
    pub fn write_headers(&self, dir: &Path) -> Result<(), String> {
        let headers = self.headers()?;
        let pairs: Vec<(&str, &str)> = headers.iter().map(|h| (h.path, h.sha256)).collect();
        self.write(dir, &pairs)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tiny() -> Vec<u8> {
        let header = b"#define X 1\n".to_vec();
        let digest = sha256::hex(&sha256::sha256(&header));
        let tables = [
            ("release.tsv", "key\tvalue\narchive\tglibc-9.9.tar.xz\narchive_sha256\tx\nbuilder.gcc\t1\nbuilder.pie\tno\n".to_string()),
            ("objects.tsv", "target\tmember\tobject\tkind\tdirectory\tcommand\ncrti.o\t0\tcsu/crti.o\tcompile\tcsu\tgcc x\n".to_string()),
            ("inputs.tsv", "object\tclass\tpath\tsha256\n".to_string()),
            ("files.tsv", format!("class\tpath\tsha256\tsize\narchive\tcsu/x.h\t{digest}\t12\ncompiler\t/usr/x.h\t\t\n")),
            ("headers.tsv", format!("path\tsha256\tsize\nx.h\t{digest}\t12\n")),
            ("interfaces.tsv", "object\tkind\tvalue\n".to_string()),
        ];
        let mut entries: Vec<(String, Vec<u8>, Option<String>)> = tables
            .iter()
            .map(|(t, text)| (format!("9.9/{t}"), text.clone().into_bytes(), None))
            .collect();
        entries.push((format!("blobs/{digest}"), header, None));
        encode(&entries).unwrap()
    }

    #[test]
    fn reads_and_writes_a_release() {
        let sources = Sources::parse(tiny()).unwrap();
        assert_eq!(sources.releases(), ["9.9"]);
        let release = sources.release("9.9").unwrap();
        assert_eq!(release.value("builder.pie"), Some("no"));
        assert_eq!(release.objects().unwrap()[0].directory, "csu");
        assert_eq!(release.files().unwrap()[1].size, None);
        let dir = std::env::temp_dir().join(format!("gps-test-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir(&dir).unwrap();
        release.write_sources(&dir.join("s")).unwrap();
        release.write_headers(&dir.join("h")).unwrap();
        assert_eq!(
            fs::read(dir.join("s/src/csu/x.h")).unwrap(),
            b"#define X 1\n"
        );
        assert_eq!(fs::read(dir.join("h/x.h")).unwrap(), b"#define X 1\n");
        assert!(
            release.write_headers(&dir.join("h")).is_err(),
            "must be new"
        );
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn stores_later_versions_as_deltas() {
        let old: Vec<u8> = (0..200)
            .flat_map(|i| format!("row {i}\n").into_bytes())
            .collect();
        let mut new = old.clone();
        new.extend_from_slice(b"row 200\n");
        let entries = vec![
            ("a/1".to_string(), old.clone(), None),
            ("a/2".to_string(), new.clone(), Some("a/1".to_string())),
        ];
        let pack = encode(&entries).unwrap();
        assert!(pack.len() < old.len() + 300, "the second entry is a delta");
        let text = String::from_utf8_lossy(&pack);
        assert!(text.contains("\ta/1\n"));
        let sources = Sources::parse(pack.clone()).unwrap();
        let read: Vec<_> = sources.entries().collect();
        assert_eq!(read, [("a/1", old.as_slice()), ("a/2", new.as_slice())]);
        // A base must come earlier.
        let reversed = vec![entries[1].clone(), entries[0].clone()];
        assert!(encode(&reversed).is_err());
    }

    #[test]
    fn rejects_corrupt_packs() {
        let good = tiny();
        let text = String::from_utf8(good.clone()).unwrap();
        let mut bad_hash = good.clone();
        *bad_hash.last_mut().unwrap() ^= 1;
        let missing_blob = {
            let start = text.find("blobs/").unwrap();
            let line_end = start + text[start..].find('\n').unwrap() + 1;
            let body = good.len() - 12;
            let mut v = text[..start].replace("\n7\n", "\n6\n").into_bytes();
            v.extend_from_slice(&good[line_end..body]);
            v
        };
        for bad in [
            bad_hash,
            missing_blob,
            good[..good.len() - 1].to_vec(),
            [good.as_slice(), b"x"].concat(),
            text.replace(FORMAT, "glibc-provider-pack 2").into_bytes(),
            text.replacen("9.9/files.tsv", "../files.tsv", 1)
                .into_bytes(),
        ] {
            assert!(Sources::parse(bad).is_err());
        }
    }
}
