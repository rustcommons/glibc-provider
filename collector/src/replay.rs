//! Rebuilds a release's startup objects and nonshared archives from the
//! recorded inputs with another compiler (Clang), and compares the result with
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
//! 4. With --compare, compares every object and target with WORK's container
//!    build: defined and undefined global symbols, allocated sections, and
//!    relocations must match; identical allocated bytes are reported separately.
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
#[derive(Debug, Default, PartialEq, Eq)]
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

fn interface(object: &Path) -> Result<(Interface, Vec<u8>), String> {
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
        bytes.extend(
            dump.lines()
                .skip(1)
                .collect::<Vec<_>>()
                .join("\n")
                .into_bytes(),
        );
    }
    Ok((result, bytes))
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
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        let mut value = |name: &str| args.next().ok_or_else(|| format!("{name} needs a value"));
        match arg.as_str() {
            "--release" => release = Some(value("--release")?),
            "--compiler" => compiler = Some(value("--compiler")?),
            "--out" => out = Some(PathBuf::from(value("--out")?)),
            "--root" => root = PathBuf::from(value("--root")?),
            "--compare" => compare = Some(PathBuf::from(value("--compare")?)),
            _ => return fail(format!("unknown argument {arg}")),
        }
    }
    let (Some(release), Some(compiler), Some(out)) = (release, compiler, out) else {
        return fail("usage: glibc-provider-replay --release X.Y --compiler PATH --out DIR [--root DIR] [--compare WORK]");
    };
    let data = root.join("data");
    let dir = data.join(&release);
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
    if let Some(work) = compare {
        for object in &built {
            let (container, replayed) =
                (work.join("source-build").join(object), build.join(object));
            if !replayed.exists() {
                continue;
            }
            let (a, a_bytes) = interface(&container)?;
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
