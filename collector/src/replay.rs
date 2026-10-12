//! Rebuilds a release's startup objects and nonshared archives from the
//! recorded inputs with a local compiler, and compares the result with
//! the container-built objects.
//!
//! glibc-provider-replay --release X.Y --compiler PATH --out DIR [--root DIR] [--compare WORK]
//!
//! 1. Recreates the recorded archive files, generated headers and kernel UAPI
//!    headers under DIR from `data/blobs/`, checking every SHA-256.
//! 2. Runs every recorded compile command with `gcc` replaced by PATH, from the
//!    recorded source subdirectory, with `-nostdinc`, the compiler's own
//!    built-in headers, and the recorded UAPI headers, so no host header is used.
//!    Dependency-file options are dropped.
//! 3. Repeats the recorded relocatable links and builds each archive in the
//!    recorded member order (`ar rcD`).
//! 4. Compares every object with the container build: defined and undefined
//!    global symbols, allocated sections, and references must match; identical
//!    allocated bytes are reported separately. The reference is the release's
//!    recorded `interfaces.tsv`, or WORK's build tree with --compare.
//!
//! glibc-provider-replay --release X.Y --record WORK [--root DIR]
//!
//! records `interfaces.tsv` from WORK's container-built objects.
//!
//! Writes DIR/report.tsv and exits nonzero if anything fails to build or differs.

#[path = "sha256.rs"]
mod sha256;

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

fn fail<T>(message: impl Into<String>) -> Result<T, String> {
    Err(message.into())
}

fn rows(path: &Path) -> Result<Vec<Vec<String>>, String> {
    let text =
        fs::read_to_string(path).map_err(|e| format!("cannot read {}: {e}", path.display()))?;
    Ok(text
        .lines()
        .skip(1)
        .map(|l| l.split('\t').map(String::from).collect())
        .collect())
}

fn run(program: &str, args: &[String], dir: &Path) -> Result<String, String> {
    let output = Command::new(program)
        .args(args)
        .current_dir(dir)
        .output()
        .map_err(|e| format!("cannot run {program}: {e}"))?;
    if !output.status.success() {
        return fail(format!(
            "{program} failed in {}: {}",
            dir.display(),
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

/// Whether the compiler accepts FLAG when assembling an empty file.
fn accepts(compiler: &str, flag: &str, scratch: &Path) -> Result<bool, String> {
    let (source, object) = (scratch.join("probe.s"), scratch.join("probe.o"));
    fs::write(&source, "").map_err(|e| e.to_string())?;
    let args = [
        "-c".to_string(),
        source.to_string_lossy().into_owned(),
        "-o".into(),
        object.to_string_lossy().into_owned(),
        flag.into(),
    ];
    let accepted = run(compiler, &args, Path::new(".")).is_ok();
    let _ = fs::remove_file(&source);
    let _ = fs::remove_file(&object);
    Ok(accepted)
}

/// Flags that make the builder's implicit code-generation defaults explicit
/// and switch off the replaying compiler's own (distribution) defaults, for
/// whatever a recorded command does not already set.
fn defaults_flags(command: &str, release: &BTreeMap<String, String>) -> Vec<String> {
    let tokens: Vec<&str> = command.split_whitespace().collect();
    let has = |prefixes: &[&str]| {
        tokens
            .iter()
            .any(|t| prefixes.iter().any(|p| t.starts_with(p)))
    };
    let builder = |key: &str| {
        release
            .get(&format!("builder.{key}"))
            .is_some_and(|v| v == "yes")
    };
    let mut flags = Vec::new();
    if !has(&[
        "-fPIC", "-fpic", "-fPIE", "-fpie", "-fno-pic", "-fno-PIC", "-fno-pie", "-fno-PIE",
    ]) {
        flags.push(if builder("pie") { "-fPIE" } else { "-fno-pie" }.to_string());
    }
    if !has(&["-fstack-protector", "-fno-stack-protector"]) && !builder("stack_protector") {
        flags.push("-fno-stack-protector".into());
    }
    if !has(&["-fcf-protection"]) && !builder("cet") {
        flags.push("-fcf-protection=none".into());
    }
    if !builder("fortify") {
        flags.push("-U_FORTIFY_SOURCE".into());
    }
    if !has(&["-fstack-clash-protection", "-fno-stack-clash-protection"]) {
        flags.push("-fno-stack-clash-protection".into());
    }
    flags
}

/// Translates a recorded GCC command for another compiler.
fn translate(
    command: &str,
    compiler: &str,
    src: &str,
    build: &str,
    extra: &[String],
) -> Vec<String> {
    let mut tokens: Vec<String> = command
        .split_whitespace()
        .map(|t| t.replace("@SRC@", src).replace("@BUILD@", build))
        .collect();
    tokens[0] = compiler.into();
    let mut out = Vec::new();
    let mut skip = false;
    for token in tokens {
        if skip {
            skip = false;
            continue;
        }
        match token.as_str() {
            "-MD" | "-MP" => {}
            "-MF" | "-MT" => skip = true,
            _ => out.push(token),
        }
    }
    out.extend(extra.iter().cloned());
    out
}

/// What must match between two relocatable objects.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct Interface {
    /// Defined global/weak symbols: name, type, binding, visibility, section.
    defined: BTreeSet<String>,
    /// Undefined global/weak symbols: name, binding.
    undefined: BTreeSet<String>,
    /// Allocated sections (name, flags), excluding notes about the stack.
    sections: BTreeSet<String>,
    /// References outside debug sections: (section, form, symbol). Counts are
    /// codegen (duplicated call sites), not interface, and are not kept.
    relocations: BTreeSet<(String, String, String)>,
}

/// The form of a relocation, merging encodings the linker resolves alike:
/// newer assemblers emit `PLT32` for calls where older ones emitted `PC32`
/// (binutils 2.31), and relaxable `GOTPCRELX` forms where older ones emitted
/// `GOTPCREL` (binutils 2.26).
fn relocation_form(kind: &str) -> String {
    match kind {
        "R_X86_64_PC32" | "R_X86_64_PLT32" => "pc-relative".into(),
        "R_X86_64_GOTPCREL" | "R_X86_64_GOTPCRELX" | "R_X86_64_REX_GOTPCRELX" => "got".into(),
        other => other.into(),
    }
}

fn interface(object: &Path) -> Result<(Interface, String), String> {
    let path = object.to_string_lossy().to_string();
    let here = Path::new(".");
    let headers = run("readelf", &["-W".into(), "-S".into(), path.clone()], here)?;
    let mut names: BTreeMap<String, String> = BTreeMap::new();
    let mut result = Interface::default();
    let mut alloc = Vec::new();
    for line in headers.lines() {
        let Some(rest) = line.trim_start().strip_prefix('[') else {
            continue;
        };
        let Some((index, rest)) = rest.split_once(']') else {
            continue;
        };
        let fields: Vec<&str> = rest.split_whitespace().collect();
        if fields.len() < 7 || fields[0] == "Name" {
            continue;
        }
        let index = index.trim().to_string();
        names.insert(index, fields[0].to_string());
        // Name Type Address Off Size ES Flg Lk Inf Al
        let flags = if fields.len() >= 10 { fields[6] } else { "" };
        // Empty sections carry nothing; GNU as emits empty .data/.bss, Clang does not.
        let empty = u64::from_str_radix(fields[4], 16).is_ok_and(|size| size == 0);
        if flags.contains('A') && fields[0] != ".note.GNU-stack" && !empty {
            result.sections.insert(format!("{} {}", fields[0], flags));
            alloc.push(fields[0].to_string());
        }
    }
    let symbols = run("readelf", &["-W".into(), "-s".into(), path.clone()], here)?;
    for line in symbols.lines() {
        let f: Vec<&str> = line.split_whitespace().collect();
        if f.len() < 8 || !f[0].ends_with(':') || f[0] == "Num:" {
            continue;
        }
        let (kind, bind, vis, ndx, name) = (f[3], f[4], f[5], f[6], f[7]);
        if bind == "LOCAL" {
            continue;
        }
        if ndx == "UND" {
            // Older GNU as references the linker-defined GOT symbol; Clang does not.
            if name != "_GLOBAL_OFFSET_TABLE_" {
                result.undefined.insert(format!("{name} {bind}"));
            }
        } else {
            let section = names.get(ndx).cloned().unwrap_or_else(|| ndx.to_string());
            result
                .defined
                .insert(format!("{name} {kind} {bind} {vis} {section}"));
        }
    }
    let relocations = run("readelf", &["-W".into(), "-r".into(), path.clone()], here)?;
    let mut section = String::new();
    for line in relocations.lines() {
        if let Some(rest) = line.strip_prefix("Relocation section '") {
            section = rest.split('\'').next().unwrap_or("").to_string();
            continue;
        }
        if section.contains(".debug") || section.is_empty() {
            continue;
        }
        let f: Vec<&str> = line.split_whitespace().collect();
        if f.len() < 3 || !f[2].starts_with("R_X86_64") {
            continue;
        }
        let symbol = f
            .get(4)
            .map(|s| s.split('+').next().unwrap_or(s))
            .unwrap_or("");
        result
            .relocations
            .insert((section.clone(), relocation_form(f[2]), symbol.to_string()));
    }
    let mut bytes = Vec::new();
    for name in alloc {
        let dump = run(
            "readelf",
            &["-W".into(), "-x".into(), name.clone(), path.clone()],
            here,
        )?;
        bytes.extend(name.as_bytes());
        bytes.push(0);
        bytes.extend(hex_dump_bytes(&dump));
    }
    Ok((result, sha256::hex(&sha256::sha256(&bytes))))
}

/// The bytes of a `readelf -x` dump: each data line is `  0xADDRESS ` followed
/// by up to four groups of eight hex digits in a fixed 35-column field, then
/// the ASCII rendering.
fn hex_dump_bytes(dump: &str) -> Vec<u8> {
    let mut bytes = Vec::new();
    for line in dump.lines() {
        let Some(rest) = line.trim_start().strip_prefix("0x") else {
            continue;
        };
        let Some((_, data)) = rest.split_once(' ') else {
            continue;
        };
        let field: String = data
            .chars()
            .take(35)
            .filter(|c| !c.is_whitespace())
            .collect();
        bytes.extend(
            (0..field.len() / 2)
                .filter_map(|i| u8::from_str_radix(&field[2 * i..2 * i + 2], 16).ok()),
        );
    }
    bytes
}

/// An object's interface as `interfaces.tsv` rows (without the object column):
/// kind and value, with the SHA-256 of the allocated bytes as kind `bytes`.
fn interface_rows(interface: &Interface, digest: &str) -> Vec<String> {
    let mut rows = Vec::new();
    rows.extend(interface.defined.iter().map(|v| format!("defined\t{v}")));
    rows.extend(
        interface
            .undefined
            .iter()
            .map(|v| format!("undefined\t{v}")),
    );
    rows.extend(interface.sections.iter().map(|v| format!("section\t{v}")));
    rows.extend(
        interface
            .relocations
            .iter()
            .map(|(s, f, n)| format!("reference\t{s} {f} {n}")),
    );
    rows.push(format!("bytes\t{digest}"));
    rows
}

/// Reads `interfaces.tsv`: object -> (interface, allocated-bytes SHA-256).
fn read_interfaces(path: &Path) -> Result<BTreeMap<String, (Interface, String)>, String> {
    let mut map: BTreeMap<String, (Interface, String)> = BTreeMap::new();
    for row in rows(path)? {
        let [object, kind, value] = &row[..] else {
            return fail(format!("{}: malformed row {row:?}", path.display()));
        };
        let (interface, digest) = map.entry(object.clone()).or_default();
        match kind.as_str() {
            "defined" => drop(interface.defined.insert(value.clone())),
            "undefined" => drop(interface.undefined.insert(value.clone())),
            "section" => drop(interface.sections.insert(value.clone())),
            "reference" => {
                let mut parts = value.splitn(3, ' ').map(String::from);
                let (Some(s), Some(f), Some(n)) = (parts.next(), parts.next(), parts.next()) else {
                    return fail(format!("{}: malformed reference {value}", path.display()));
                };
                interface.relocations.insert((s, f, n));
            }
            "bytes" => *digest = value.clone(),
            _ => return fail(format!("{}: unknown kind {kind}", path.display())),
        }
    }
    Ok(map)
}

fn describe(a: &Interface, b: &Interface) -> String {
    let diff = |name: &str, x: &BTreeSet<String>, y: &BTreeSet<String>| -> Option<String> {
        let only_x: Vec<_> = x.difference(y).cloned().collect();
        let only_y: Vec<_> = y.difference(x).cloned().collect();
        (!only_x.is_empty() || !only_y.is_empty())
            .then(|| format!("{name}: container only {only_x:?}, replay only {only_y:?}"))
    };
    // A reference the container made directly and the replay makes through the
    // GOT resolves to the same address; the GOT form is the more general one.
    // Only references that differ are considered for this rule.
    let only_a: BTreeSet<_> = a.relocations.difference(&b.relocations).cloned().collect();
    let only_b: BTreeSet<_> = b.relocations.difference(&a.relocations).cloned().collect();
    let generalized = |r: &(String, String, String)| {
        r.1 == "pc-relative" && only_b.contains(&(r.0.clone(), "got".into(), r.2.clone()))
    };
    let container: BTreeSet<String> = only_a
        .iter()
        .filter(|r| !generalized(r))
        .map(|r| format!("{} {} {}", r.0, r.1, r.2))
        .collect();
    let replay: BTreeSet<String> = only_b
        .iter()
        .filter(|r| {
            !(r.1 == "got" && only_a.contains(&(r.0.clone(), "pc-relative".into(), r.2.clone())))
        })
        .map(|r| format!("{} {} {}", r.0, r.1, r.2))
        .collect();
    [
        diff("defined", &a.defined, &b.defined),
        diff("undefined", &a.undefined, &b.undefined),
        diff("sections", &a.sections, &b.sections),
        diff("relocations", &container, &replay),
    ]
    .into_iter()
    .flatten()
    .collect::<Vec<_>>()
    .join("; ")
}

fn replay() -> Result<bool, String> {
    let mut release = None;
    let mut compiler = None;
    let mut out = None;
    let mut root = PathBuf::from(".");
    let mut compare = None;
    let mut record = None;
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        let mut value = |name: &str| args.next().ok_or_else(|| format!("{name} needs a value"));
        match arg.as_str() {
            "--release" => release = Some(value("--release")?),
            "--compiler" => compiler = Some(value("--compiler")?),
            "--out" => out = Some(PathBuf::from(value("--out")?)),
            "--root" => root = PathBuf::from(value("--root")?),
            "--compare" => compare = Some(PathBuf::from(value("--compare")?)),
            "--record" => record = Some(PathBuf::from(value("--record")?)),
            _ => return fail(format!("unknown argument {arg}")),
        }
    }
    let usage = "usage: glibc-provider-replay --release X.Y --compiler PATH --out DIR [--root DIR] [--compare WORK]
       glibc-provider-replay --release X.Y --record WORK [--root DIR]";
    let Some(release) = release else {
        return fail(usage);
    };
    let data = root.join("data");
    let dir = data.join(&release);
    if let Some(work) = record {
        return record_interfaces(&dir, &work);
    }
    let (Some(compiler), Some(out)) = (compiler, out) else {
        return fail(usage);
    };
    let recorded_interfaces = if compare.is_none() {
        read_interfaces(&dir.join("interfaces.tsv"))?
    } else {
        BTreeMap::new()
    };
    if out.exists() {
        return fail(format!("{} must not exist", out.display()));
    }
    let out = {
        fs::create_dir_all(&out).map_err(|e| e.to_string())?;
        out.canonicalize().map_err(|e| e.to_string())?
    };
    let (src, build, include) = (out.join("src"), out.join("build"), out.join("include"));
    for row in rows(&dir.join("files.tsv"))? {
        let (class, path, digest) = (&row[0], &row[1], &row[2]);
        let base = match class.as_str() {
            "archive" => &src,
            "generated" => &build,
            "uapi" => &include,
            _ => continue,
        };
        let bytes = fs::read(data.join("blobs").join(&digest[..2]).join(digest))
            .map_err(|e| e.to_string())?;
        if sha256::hex(&sha256::sha256(&bytes)) != *digest {
            return fail(format!("blob for {path} does not match its SHA-256"));
        }
        let target = base.join(path);
        fs::create_dir_all(target.parent().unwrap()).map_err(|e| e.to_string())?;
        fs::write(&target, bytes).map_err(|e| e.to_string())?;
    }
    // Only the compiler's own headers and the recorded UAPI headers are visible.
    let clang = run(&compiler, &["--version".into()], Path::new("."))?.contains("clang");
    let mut extra = Vec::new();
    if clang {
        let resource = run(&compiler, &["-print-resource-dir".into()], Path::new("."))?;
        extra.extend([
            "--target=x86_64-linux-gnu".to_string(),
            "-nostdinc".into(),
            "-isystem".into(),
            format!("{}/include", resource.trim()),
            // Clang drops attributes declared after a definition, where GCC applies them.
            "-Werror=ignored-attributes".into(),
        ]);
    } else {
        extra.push("-nostdinc".into());
        for dir in ["include", "include-fixed"] {
            let path = run(
                &compiler,
                &[format!("-print-file-name={dir}")],
                Path::new("."),
            )?;
            extra.extend(["-isystem".to_string(), path.trim().to_string()]);
        }
    }
    // Some distributions (Fedora) build GNU as to add an x86 "ISA used" note to
    // every object; the builders' assemblers did not. Older assemblers lack the
    // option, so it is passed only where accepted.
    let used_note = "-Wa,-mx86-used-note=no".to_string();
    if !clang && accepts(&compiler, &used_note, &out)? {
        extra.push(used_note);
    }
    extra.extend([
        "-isystem".to_string(),
        include.to_string_lossy().into_owned(),
    ]);
    // A relocatable link adds nothing of its own: no PIE, no build ID.
    let mut link_extra: Vec<String> = extra.iter().take(usize::from(clang)).cloned().collect();
    link_extra.extend(["-no-pie".to_string(), "-Wl,--build-id=none".into()]);
    let recorded: BTreeMap<String, String> = rows(&dir.join("release.tsv"))?
        .into_iter()
        .filter_map(|r| Some((r.first()?.clone(), r.get(1)?.clone())))
        .collect();
    if !recorded.contains_key("builder.pie") {
        return fail("release.tsv lacks the builder defaults; recollect the release");
    }
    let (s, b) = (
        src.to_string_lossy().into_owned(),
        build.to_string_lossy().into_owned(),
    );
    let objects = rows(&dir.join("objects.tsv"))?;
    // Reviewed differences: release, object, compiler family, exact difference, reason.
    let accepted_path = root.join("qualification/accepted-differences.tsv");
    let accepted = if accepted_path.exists() {
        rows(&accepted_path)?
    } else {
        Vec::new()
    };
    let mut report: Vec<String> = vec!["object\tstatus\tdetail".into()];
    let mut ok = true;
    let mut built = BTreeSet::new();
    // Compile first, then relocatable links (their inputs must exist).
    for pass in ["compile", "relocatable"] {
        for row in objects.iter().filter(|r| r[3] == pass) {
            let (object, subdir, command) = (&row[2], &row[4], &row[5]);
            if !built.insert(object.clone()) {
                continue;
            }
            let cwd = if subdir == "." {
                src.clone()
            } else {
                src.join(subdir)
            };
            fs::create_dir_all(&cwd).map_err(|e| e.to_string())?;
            if let Some(parent) = build.join(object).parent() {
                fs::create_dir_all(parent).map_err(|e| e.to_string())?;
            }
            let mut flags = if pass == "compile" {
                extra.clone()
            } else {
                link_extra.clone()
            };
            if pass == "compile" {
                flags.extend(defaults_flags(command, &recorded));
            }
            let args = translate(command, &compiler, &s, &b, &flags);
            if let Err(e) = run(&args[0], &args[1..], &cwd) {
                ok = false;
                report.push(format!(
                    "{object}\tbuild-failed\t{}",
                    e.replace(['\t', '\n'], " ")
                ));
            }
        }
    }
    let mut targets: BTreeMap<String, Vec<(usize, String)>> = BTreeMap::new();
    for row in objects.iter().filter(|r| r[0].ends_with(".a")) {
        targets
            .entry(row[0].clone())
            .or_default()
            .push((row[1].parse().unwrap_or(0), row[2].clone()));
    }
    for (archive, mut members) in targets {
        members.sort();
        members.dedup();
        let path = build.join(&archive);
        let mut args = vec!["rcD".to_string(), path.to_string_lossy().into_owned()];
        args.extend(
            members
                .iter()
                .map(|(_, m)| build.join(m).to_string_lossy().into_owned()),
        );
        if let Err(e) = run("ar", &args, Path::new(".")) {
            ok = false;
            report.push(format!(
                "{archive}\tbuild-failed\t{}",
                e.replace(['\t', '\n'], " ")
            ));
        }
    }
    {
        for object in &built {
            let replayed = build.join(object);
            if !replayed.exists() {
                continue;
            }
            let (a, a_bytes) = match &compare {
                Some(work) => interface(&work.join("source-build").join(object))?,
                None => match recorded_interfaces.get(object) {
                    Some(recorded) => recorded.clone(),
                    None => return fail(format!("interfaces.tsv lacks {object}")),
                },
            };
            let (r, r_bytes) = interface(&replayed)?;
            let difference = describe(&a, &r);
            let family = if clang { "clang" } else { "gcc" };
            let reviewed = accepted.iter().any(|row| {
                row.len() >= 5
                    && row[0] == release
                    && row[1] == *object
                    && (row[2] == family || row[2] == "any")
                    && row[3] == difference
            });
            let status = if !difference.is_empty() && reviewed {
                "accepted"
            } else if !difference.is_empty() {
                ok = false;
                "differs"
            } else if a_bytes == r_bytes {
                "identical-bytes"
            } else {
                "equivalent"
            };
            report.push(format!(
                "{object}\t{status}\t{}",
                difference.replace(['\t', '\n'], " ")
            ));
        }
    }
    fs::write(out.join("report.tsv"), report.join("\n") + "\n").map_err(|e| e.to_string())?;
    let mut counts: BTreeMap<&str, usize> = BTreeMap::new();
    for line in &report[1..] {
        *counts
            .entry(line.split('\t').nth(1).unwrap_or(""))
            .or_default() += 1;
    }
    println!("glibc {release} with {compiler}: {counts:?}");
    Ok(ok)
}

/// Records the interface of every container-built object in DIR/interfaces.tsv,
/// the reference a replay is compared with when no build tree is at hand.
fn record_interfaces(dir: &Path, work: &Path) -> Result<bool, String> {
    let mut objects: Vec<String> = rows(&dir.join("objects.tsv"))?
        .into_iter()
        .map(|r| r[2].clone())
        .collect();
    objects.sort();
    objects.dedup();
    let mut lines = vec!["object	kind	value".to_string()];
    for object in objects {
        let (interface, digest) = interface(&work.join("source-build").join(&object))?;
        lines.extend(
            interface_rows(&interface, &digest)
                .into_iter()
                .map(|row| format!("{object}	{row}")),
        );
    }
    fs::write(
        dir.join("interfaces.tsv"),
        lines.join(
            "
",
        ) + "
",
    )
    .map_err(|e| e.to_string())?;
    Ok(true)
}

fn main() {
    match replay() {
        Ok(true) => {}
        Ok(false) => std::process::exit(2),
        Err(message) => {
            eprintln!("glibc-provider-replay: {message}");
            std::process::exit(1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compares_references_not_encodings() {
        let refs = |items: &[(&str, &str)]| Interface {
            relocations: items
                .iter()
                .map(|(kind, sym)| {
                    (
                        ".rela.text".to_string(),
                        relocation_form(kind),
                        sym.to_string(),
                    )
                })
                .collect(),
            ..Interface::default()
        };
        // Same references in older and newer encodings.
        let old = refs(&[("R_X86_64_GOTPCREL", "g"), ("R_X86_64_PC32", "f")]);
        let new = refs(&[("R_X86_64_REX_GOTPCRELX", "g"), ("R_X86_64_PLT32", "f")]);
        assert_eq!(describe(&old, &new), "");
        // A direct reference made through the GOT instead is the general form.
        assert_eq!(
            describe(
                &refs(&[("R_X86_64_PC32", "h")]),
                &refs(&[("R_X86_64_GOTPCRELX", "h")])
            ),
            ""
        );
        // The reverse is not accepted, nor is a missing reference.
        assert_ne!(
            describe(
                &refs(&[("R_X86_64_GOTPCREL", "h")]),
                &refs(&[("R_X86_64_PC32", "h")])
            ),
            ""
        );
        assert_ne!(describe(&refs(&[("R_X86_64_PC32", "h")]), &refs(&[])), "");
    }

    #[test]
    fn reads_hex_dumps_without_the_ascii_column() {
        let dump = "\nHex dump of section '.rodata.cst4':\n  0x00000000 01000200 deadbeef 31ed4989 ab       ........1.I..\n  0x00000010 cafe                                ..\n";
        assert_eq!(
            hex_dump_bytes(dump),
            [1, 0, 2, 0, 0xde, 0xad, 0xbe, 0xef, 0x31, 0xed, 0x49, 0x89, 0xab, 0xca, 0xfe]
        );
    }

    #[test]
    fn recorded_interfaces_read_back_unchanged() {
        let interface = Interface {
            defined: ["_start FUNC GLOBAL DEFAULT .text".to_string()].into(),
            undefined: ["main GLOBAL".to_string()].into(),
            sections: [".text AX".to_string()].into(),
            relocations: [(
                ".rela.text".to_string(),
                "got".to_string(),
                "__libc_start_main".to_string(),
            )]
            .into(),
        };
        let dir = std::env::temp_dir().join(format!("replay-test-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("interfaces.tsv");
        let lines: Vec<String> = std::iter::once("object\tkind\tvalue".to_string())
            .chain(
                interface_rows(&interface, "ab")
                    .into_iter()
                    .map(|r| format!("csu/start.o\t{r}")),
            )
            .collect();
        fs::write(&path, lines.join("\n") + "\n").unwrap();
        let read = read_interfaces(&path).unwrap();
        fs::remove_dir_all(&dir).unwrap();
        assert_eq!(read["csu/start.o"], (interface, "ab".to_string()));
    }

    #[test]
    fn makes_builder_defaults_explicit_only_where_unset() {
        let pie: BTreeMap<String, String> = [("builder.pie".to_string(), "yes".to_string())].into();
        let none = BTreeMap::new();
        assert!(defaults_flags("gcc x.c -c", &pie).contains(&"-fPIE".to_string()));
        assert!(defaults_flags("gcc x.c -c", &none).contains(&"-fno-pie".to_string()));
        let pic = defaults_flags("gcc x.c -c -fPIC -fno-stack-protector", &none);
        assert!(!pic
            .iter()
            .any(|f| f.contains("pie") || f.contains("stack-protector")));
        assert!(pic.contains(&"-U_FORTIFY_SOURCE".to_string()));
    }

    #[test]
    fn translates_commands_for_another_compiler() {
        let args = translate(
            "gcc ../x.S -c -I@BUILD@/csu -o @BUILD@/csu/x.o -MD -MP -MF @BUILD@/csu/x.o.dt -MT @BUILD@/csu/x.o",
            "clang",
            "/o/src",
            "/o/build",
            &["-nostdinc".into()],
        );
        assert_eq!(
            args,
            [
                "clang",
                "../x.S",
                "-c",
                "-I/o/build/csu",
                "-o",
                "/o/build/csu/x.o",
                "-nostdinc"
            ]
        );
    }
}
