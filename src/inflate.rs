//! Decoding of one gzip member (RFC 1952) holding DEFLATE data (RFC 1951),
//! std-only. Stored, fixed-Huffman and dynamic-Huffman blocks are supported;
//! the CRC-32 and size in the gzip trailer are checked.

const MAX_BITS: usize = 15;
const LENGTH_BASE: [u16; 29] = [
    3, 4, 5, 6, 7, 8, 9, 10, 11, 13, 15, 17, 19, 23, 27, 31, 35, 43, 51, 59, 67, 83, 99, 115, 131,
    163, 195, 227, 258,
];
const LENGTH_EXTRA: [u8; 29] = [
    0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 2, 2, 2, 2, 3, 3, 3, 3, 4, 4, 4, 4, 5, 5, 5, 5, 0,
];
const DISTANCE_BASE: [u16; 30] = [
    1, 2, 3, 4, 5, 7, 9, 13, 17, 25, 33, 49, 65, 97, 129, 193, 257, 385, 513, 769, 1025, 1537,
    2049, 3073, 4097, 6145, 8193, 12289, 16385, 24577,
];
const DISTANCE_EXTRA: [u8; 30] = [
    0, 0, 0, 0, 1, 1, 2, 2, 3, 3, 4, 4, 5, 5, 6, 6, 7, 7, 8, 8, 9, 9, 10, 10, 11, 11, 12, 12, 13,
    13,
];
/// Order in which code-length code lengths are sent.
const CODE_LENGTH_ORDER: [usize; 19] = [
    16, 17, 18, 0, 8, 7, 9, 6, 10, 5, 11, 4, 12, 3, 13, 2, 14, 1, 15,
];

struct Bits<'a> {
    data: &'a [u8],
    next: usize,
    buffer: u64,
    count: u32,
}

impl<'a> Bits<'a> {
    fn new(data: &'a [u8]) -> Self {
        Self {
            data,
            next: 0,
            buffer: 0,
            count: 0,
        }
    }

    fn refill(&mut self) {
        while self.count <= 56 && self.next < self.data.len() {
            self.buffer |= u64::from(self.data[self.next]) << self.count;
            self.next += 1;
            self.count += 8;
        }
    }

    /// The next `n` bits (at most 32), without consuming them; zero past the end.
    fn peek(&mut self, n: u32) -> u32 {
        self.refill();
        (self.buffer & ((1u64 << n) - 1)) as u32
    }

    fn consume(&mut self, n: u32) -> Result<(), String> {
        if n > self.count {
            return Err("deflate data ends early".into());
        }
        self.buffer >>= n;
        self.count -= n;
        Ok(())
    }

    fn take(&mut self, n: u32) -> Result<u32, String> {
        let value = self.peek(n);
        self.consume(n)?;
        Ok(value)
    }

    /// Drops the bits up to the next byte boundary.
    fn align(&mut self) {
        let drop = self.count % 8;
        self.buffer >>= drop;
        self.count -= drop;
    }

    /// Bytes consumed so far, counting buffered whole bytes as unread.
    fn position(&self) -> usize {
        self.next - (self.count / 8) as usize
    }
}

/// A canonical Huffman code as a direct lookup table on `MAX_BITS` bits:
/// each entry is `symbol << 4 | length`, zero where no code matches.
struct Huffman {
    table: Vec<u32>,
}

impl Huffman {
    fn new(lengths: &[u8]) -> Result<Self, String> {
        let mut count = [0u32; MAX_BITS + 1];
        for &length in lengths {
            count[usize::from(length)] += 1;
        }
        count[0] = 0;
        let mut left = 1i64;
        for &n in &count[1..] {
            left = (left << 1) - i64::from(n);
            if left < 0 {
                return Err("over-subscribed Huffman code".into());
            }
        }
        let mut next = [0u32; MAX_BITS + 2];
        for bits in 1..=MAX_BITS {
            next[bits + 1] = (next[bits] + count[bits]) << 1;
        }
        let mut table = vec![0u32; 1 << MAX_BITS];
        for (symbol, &length) in lengths.iter().enumerate() {
            if length == 0 {
                continue;
            }
            let length = usize::from(length);
            let code = next[length];
            next[length] += 1;
            // Codes are sent most significant bit first; the table is indexed LSB first.
            let reversed = (code.reverse_bits() >> (32 - length)) as usize;
            let entry = (symbol as u32) << 4 | length as u32;
            let mut index = reversed;
            while index < table.len() {
                table[index] = entry;
                index += 1 << length;
            }
        }
        Ok(Self { table })
    }

    fn decode(&self, bits: &mut Bits) -> Result<usize, String> {
        let entry = self.table[bits.peek(MAX_BITS as u32) as usize];
        if entry & 15 == 0 {
            return Err("invalid Huffman code".into());
        }
        bits.consume(entry & 15)?;
        Ok((entry >> 4) as usize)
    }
}

fn fixed() -> (Huffman, Huffman) {
    let mut lengths = [0u8; 288];
    lengths[..144].fill(8);
    lengths[144..256].fill(9);
    lengths[256..280].fill(7);
    lengths[280..].fill(8);
    (
        Huffman::new(&lengths).expect("fixed code"),
        Huffman::new(&[5; 30]).expect("fixed code"),
    )
}

fn dynamic(bits: &mut Bits) -> Result<(Huffman, Huffman), String> {
    let literals = bits.take(5)? as usize + 257;
    let distances = bits.take(5)? as usize + 1;
    let code_lengths = bits.take(4)? as usize + 4;
    if literals > 286 || distances > 30 {
        return Err("too many Huffman codes".into());
    }
    let mut lengths = [0u8; 19];
    for &index in &CODE_LENGTH_ORDER[..code_lengths] {
        lengths[index] = bits.take(3)? as u8;
    }
    let code = Huffman::new(&lengths)?;
    let mut lengths = vec![0u8; literals + distances];
    let mut index = 0;
    while index < lengths.len() {
        let symbol = code.decode(bits)?;
        let (value, repeat) = match symbol {
            0..=15 => (symbol as u8, 1),
            16 => {
                let previous = *lengths[..index].last().ok_or("repeat with no length")?;
                (previous, 3 + bits.take(2)? as usize)
            }
            17 => (0, 3 + bits.take(3)? as usize),
            _ => (0, 11 + bits.take(7)? as usize),
        };
        if index + repeat > lengths.len() {
            return Err("too many code lengths".into());
        }
        lengths[index..index + repeat].fill(value);
        index += repeat;
    }
    if lengths[256] == 0 {
        return Err("no end-of-block code".into());
    }
    Ok((
        Huffman::new(&lengths[..literals])?,
        Huffman::new(&lengths[literals..])?,
    ))
}

/// Decodes raw DEFLATE data; returns the output and the bytes of input used.
fn inflate(data: &[u8], limit: usize) -> Result<(Vec<u8>, usize), String> {
    let mut bits = Bits::new(data);
    let mut out = Vec::new();
    loop {
        let last = bits.take(1)? == 1;
        match bits.take(2)? {
            0 => {
                bits.align();
                let length = bits.take(16)?;
                if length != !bits.take(16)? & 0xffff {
                    return Err("stored block length check failed".into());
                }
                for _ in 0..length {
                    out.push(bits.take(8)? as u8);
                }
            }
            kind @ (1 | 2) => {
                let (literal, distance) = if kind == 1 {
                    fixed()
                } else {
                    dynamic(&mut bits)?
                };
                loop {
                    let symbol = literal.decode(&mut bits)?;
                    if symbol < 256 {
                        out.push(symbol as u8);
                        continue;
                    }
                    if symbol == 256 {
                        break;
                    }
                    let index = symbol - 257;
                    if index >= LENGTH_BASE.len() {
                        return Err("invalid length symbol".into());
                    }
                    let length = usize::from(LENGTH_BASE[index])
                        + bits.take(u32::from(LENGTH_EXTRA[index]))? as usize;
                    let index = distance.decode(&mut bits)?;
                    if index >= DISTANCE_BASE.len() {
                        return Err("invalid distance symbol".into());
                    }
                    let back = usize::from(DISTANCE_BASE[index])
                        + bits.take(u32::from(DISTANCE_EXTRA[index]))? as usize;
                    if back > out.len() {
                        return Err("distance reaches before the start".into());
                    }
                    let start = out.len() - back;
                    for offset in 0..length {
                        out.push(out[start + offset]);
                    }
                }
            }
            _ => return Err("invalid block type".into()),
        }
        if out.len() > limit {
            return Err(format!("output exceeds {limit} bytes"));
        }
        if last {
            break;
        }
    }
    bits.align();
    Ok((out, bits.position()))
}

fn crc32(data: &[u8]) -> u32 {
    let mut table = [0u32; 256];
    for (n, entry) in table.iter_mut().enumerate() {
        let mut c = n as u32;
        for _ in 0..8 {
            c = if c & 1 == 1 {
                0xedb8_8320 ^ (c >> 1)
            } else {
                c >> 1
            };
        }
        *entry = c;
    }
    !data.iter().fold(!0u32, |c, &b| {
        table[((c ^ u32::from(b)) & 0xff) as usize] ^ (c >> 8)
    })
}

/// Decodes a single gzip member of at most `limit` decoded bytes.
pub fn gunzip(data: &[u8], limit: usize) -> Result<Vec<u8>, String> {
    if data.len() < 18 || data[..3] != [0x1f, 0x8b, 8] {
        return Err("not gzip data".into());
    }
    let flags = data[3];
    if flags & 0xe0 != 0 {
        return Err("reserved gzip flags set".into());
    }
    let mut position = 10;
    let skip_to = |position: &mut usize, end: Option<usize>| -> Result<(), String> {
        let next = match end {
            Some(n) => *position + n,
            None => {
                *position
                    + data[*position..]
                        .iter()
                        .position(|&b| b == 0)
                        .ok_or("unterminated gzip header field")?
                    + 1
            }
        };
        if next > data.len() {
            return Err("gzip header ends early".into());
        }
        *position = next;
        Ok(())
    };
    if flags & 4 != 0 {
        let extra = usize::from(u16::from_le_bytes([data[10], data[11]]));
        skip_to(&mut position, Some(2 + extra))?;
    }
    for flag in [8, 16] {
        if flags & flag != 0 {
            skip_to(&mut position, None)?;
        }
    }
    if flags & 2 != 0 {
        skip_to(&mut position, Some(2))?;
    }
    let (out, used) = inflate(&data[position..], limit)?;
    let trailer = position + used;
    if data.len() != trailer + 8 {
        return Err("gzip data must hold exactly one member".into());
    }
    let word = |at: usize| u32::from_le_bytes(data[at..at + 4].try_into().expect("4 bytes"));
    if word(trailer) != crc32(&out) {
        return Err("gzip CRC-32 mismatch".into());
    }
    if word(trailer + 4) != out.len() as u32 {
        return Err("gzip size mismatch".into());
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    // From `printf ... | gzip -9 -n` (fixed and dynamic blocks) and Python's
    // zlib with level 0 (a stored block).
    const HELLO: &[u8] = &[
        0x1f, 0x8b, 0x08, 0x00, 0x00, 0x00, 0x00, 0x00, 0x02, 0x03, 0xcb, 0x48, 0xcd, 0xc9, 0xc9,
        0x57, 0x28, 0xcf, 0x2f, 0xca, 0x49, 0xe1, 0x02, 0x00, 0x2d, 0x3b, 0x08, 0xaf, 0x0c, 0x00,
        0x00, 0x00,
    ];

    #[test]
    fn decodes_fixed_huffman_and_checks_the_trailer() {
        assert_eq!(gunzip(HELLO, 100).unwrap(), b"hello world\n");
        let mut bad = HELLO.to_vec();
        bad[24] ^= 1;
        assert!(gunzip(&bad, 100).unwrap_err().contains("CRC"));
        for end in 0..HELLO.len() {
            assert!(gunzip(&HELLO[..end], 100).is_err(), "{end}");
        }
        assert!(gunzip(HELLO, 5).is_err());
        let mut twice = HELLO.to_vec();
        twice.extend_from_slice(HELLO);
        assert!(gunzip(&twice, 100).is_err());
    }

    #[test]
    fn decodes_dynamic_huffman_with_long_back_references() {
        // 400 numbered lines, `gzip -9 -n`: one dynamic block.
        let data = include_bytes!("testdata-dynamic.gz");
        assert_eq!((data[10] >> 1) & 3, 2);
        let out = gunzip(data, 1 << 20).unwrap();
        assert_eq!(out.len(), 22625);
        assert_eq!(
            crate::sha256::hex(&crate::sha256::sha256(&out)),
            "8719edefd60c296efa867b9f3f9b0ca0f7a2e214e8b6b7ed9fcba32749b038fc"
        );
        assert!(out.starts_with(b"line 0: the quick brown fox"));
    }

    #[test]
    fn decodes_stored_blocks_and_back_references() {
        // A stored block: header bits 001 (final, stored), then LEN/NLEN and bytes.
        let mut data = vec![0x1f, 0x8b, 8, 0, 0, 0, 0, 0, 0, 3, 0x01, 5, 0, 0xfa, 0xff];
        data.extend_from_slice(b"abcde");
        data.extend_from_slice(&crc32(b"abcde").to_le_bytes());
        data.extend_from_slice(&5u32.to_le_bytes());
        assert_eq!(gunzip(&data, 100).unwrap(), b"abcde");
        assert_eq!(crc32(b"123456789"), 0xcbf4_3926);
    }

    #[test]
    fn rejects_bad_codes() {
        assert!(Huffman::new(&[1, 1, 1]).is_err());
        let mut data = HELLO.to_vec();
        data[10] = 0xff; // block type 3
        assert!(gunzip(&data, 100).is_err());
    }
}
