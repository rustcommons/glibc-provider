//! Line deltas between two versions of a file.
//!
//! A delta is a sequence of operations on the base's lines (each with its
//! newline; the last may lack one):
//!
//! - `c<N>\n` copies the next N base lines;
//! - `s<N>\n` skips the next N base lines;
//! - `a<N>\n` followed by N bytes inserts those bytes.

use std::collections::HashMap;

fn lines(data: &[u8]) -> Vec<&[u8]> {
    let mut out = Vec::new();
    let mut start = 0;
    for (index, &byte) in data.iter().enumerate() {
        if byte == b'\n' {
            out.push(&data[start..=index]);
            start = index + 1;
        }
    }
    if start < data.len() {
        out.push(&data[start..]);
    }
    out
}

/// Rebuilds a file from its base and a delta.
pub fn apply(base: &[u8], delta: &[u8]) -> Result<Vec<u8>, String> {
    let base = lines(base);
    let mut cursor = 0;
    let mut out = Vec::new();
    let mut at = 0;
    while at < delta.len() {
        let op = delta[at];
        let end = at
            + delta[at..]
                .iter()
                .position(|&b| b == b'\n')
                .ok_or("delta operation ends early")?;
        let count: usize = std::str::from_utf8(&delta[at + 1..end])
            .ok()
            .and_then(|n| n.parse().ok())
            .ok_or("invalid delta count")?;
        at = end + 1;
        match op {
            b'c' | b's' => {
                let next = cursor + count;
                if next > base.len() {
                    return Err("delta reads past the base".into());
                }
                if op == b'c' {
                    for line in &base[cursor..next] {
                        out.extend_from_slice(line);
                    }
                }
                cursor = next;
            }
            b'a' => {
                let bytes = delta.get(at..at + count).ok_or("delta insert ends early")?;
                out.extend_from_slice(bytes);
                at += count;
            }
            _ => return Err("invalid delta operation".into()),
        }
    }
    Ok(out)
}

/// Numbers each distinct line, so lines compare as integers.
fn numbered<'a>(ids: &mut HashMap<&'a [u8], u32>, lines: &[&'a [u8]]) -> Vec<u32> {
    lines
        .iter()
        .map(|&line| {
            let next = ids.len() as u32;
            *ids.entry(line).or_insert(next)
        })
        .collect()
}

/// A delta from `base` to `new`, or `None` when they differ in more than
/// `limit` lines (the edit search is quadratic in the number of differences).
pub fn diff(base: &[u8], new: &[u8], limit: usize) -> Option<Vec<u8>> {
    let (a, b) = (lines(base), lines(new));
    let mut ids: HashMap<&[u8], u32> = HashMap::new();
    let a_ids = numbered(&mut ids, &a);
    let b_ids = numbered(&mut ids, &b);
    let script = myers(&a_ids, &b_ids, limit)?;
    // Encode runs of the script.
    let mut out = Vec::new();
    let (mut i, mut j) = (0, 0);
    let mut index = 0;
    while index < script.len() {
        let kind = script[index];
        let mut run = 0;
        while index < script.len() && script[index] == kind {
            run += 1;
            index += 1;
        }
        match kind {
            Edit::Keep => {
                out.extend_from_slice(format!("c{run}\n").as_bytes());
                i += run;
                j += run;
            }
            Edit::Delete => {
                out.extend_from_slice(format!("s{run}\n").as_bytes());
                i += run;
            }
            Edit::Insert => {
                let bytes: Vec<u8> = b[j..j + run].concat();
                out.extend_from_slice(format!("a{}\n", bytes.len()).as_bytes());
                out.extend_from_slice(&bytes);
                j += run;
            }
        }
    }
    debug_assert_eq!((i, j), (a.len(), b.len()));
    Some(out)
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Edit {
    Keep,
    Delete,
    Insert,
}

/// Myers' O(ND) shortest edit script, keeping the frontier of each step.
fn myers(a: &[u32], b: &[u32], limit: usize) -> Option<Vec<Edit>> {
    let (n, m) = (a.len() as isize, b.len() as isize);
    let max = (n + m) as usize;
    let offset = max as isize + 1;
    let mut v = vec![0isize; 2 * max + 3];
    let mut trace: Vec<Vec<isize>> = Vec::new();
    for d in 0..=max.min(limit) as isize {
        trace.push(v[(offset - d) as usize..=(offset + d) as usize].to_vec());
        let mut k = -d;
        while k <= d {
            let index = (offset + k) as usize;
            let mut x = if k == -d || (k != d && v[index - 1] < v[index + 1]) {
                v[index + 1]
            } else {
                v[index - 1] + 1
            };
            let mut y = x - k;
            while x < n && y < m && a[x as usize] == b[y as usize] {
                x += 1;
                y += 1;
            }
            v[index] = x;
            if x >= n && y >= m {
                return Some(backtrack(&trace, a.len(), b.len()));
            }
            k += 2;
        }
    }
    None
}

fn backtrack(trace: &[Vec<isize>], n: usize, m: usize) -> Vec<Edit> {
    let (mut x, mut y) = (n as isize, m as isize);
    let mut edits = Vec::new();
    for (d, v) in trace.iter().enumerate().rev() {
        let d = d as isize;
        // `v` is the frontier before step d, for diagonals -d..=d.
        let (px, py) = if d == 0 {
            (0, 0)
        } else {
            let at = |k: isize| v[(k + d) as usize];
            let k = x - y;
            let previous = if k == -d || (k != d && at(k - 1) < at(k + 1)) {
                k + 1
            } else {
                k - 1
            };
            let px = at(previous);
            (px, px - previous)
        };
        while x > px && y > py {
            edits.push(Edit::Keep);
            x -= 1;
            y -= 1;
        }
        if d > 0 {
            edits.push(if x == px { Edit::Insert } else { Edit::Delete });
        }
        x = px;
        y = py;
    }
    edits.reverse();
    edits
}

#[cfg(test)]
mod tests {
    use super::*;

    fn round_trip(base: &[u8], new: &[u8]) {
        let delta = diff(base, new, 1000).expect("within limit");
        assert_eq!(apply(base, &delta).unwrap(), new, "{delta:?}");
    }

    #[test]
    fn deltas_reproduce_every_edit() {
        let base = b"a\nb\nc\nd\ne\n".as_slice();
        for new in [
            b"a\nb\nc\nd\ne\n".as_slice(),
            b"a\nc\nd\ne\n",
            b"a\nb\nx\nc\nd\ne\n",
            b"b\nc\nd\ne\nf",
            b"",
            b"x\ny\n",
            b"a\nb\nc\nd\ne",
            b"e\nd\nc\nb\na\n",
        ] {
            round_trip(base, new);
            round_trip(new, base);
        }
        round_trip(b"", b"only\n");
        round_trip(b"}\n}\n}\n", b"}\nx\n}\n}\ny\n");
    }

    #[test]
    fn deltas_are_small_for_small_changes() {
        let base: Vec<u8> = (0..1000)
            .flat_map(|i| format!("line {i}\n").into_bytes())
            .collect();
        let mut new = base.clone();
        new.extend_from_slice(b"one more\n");
        let delta = diff(&base, &new, 10).unwrap();
        assert_eq!(delta, b"c1000\na9\none more\n");
        assert!(diff(&base, b"other\n", 10).is_none());
    }

    #[test]
    fn rejects_bad_deltas() {
        for bad in [&b"c9\n"[..], b"s1", b"a5\nab", b"x1\n", b"cz\n"] {
            assert!(apply(b"a\nb\n", bad).is_err(), "{bad:?}");
        }
    }
}
