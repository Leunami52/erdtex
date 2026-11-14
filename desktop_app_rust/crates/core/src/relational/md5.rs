//! MD5 (RFC 1321), for the short hash in long SQL constraint names, as the Python app makes
//! them.  Not for security.

const S: [u32; 64] = [
    7, 12, 17, 22, 7, 12, 17, 22, 7, 12, 17, 22, 7, 12, 17, 22, 5, 9, 14, 20, 5, 9, 14, 20, 5, 9, 14, 20, 5,
    9, 14, 20, 4, 11, 16, 23, 4, 11, 16, 23, 4, 11, 16, 23, 4, 11, 16, 23, 6, 10, 15, 21, 6, 10, 15, 21, 6,
    10, 15, 21, 6, 10, 15, 21,
];

fn table() -> [u32; 64] {
    let mut k = [0u32; 64];
    for (i, v) in k.iter_mut().enumerate() {
        *v = ((i as f64 + 1.0).sin().abs() * 4_294_967_296.0) as u32;
    }
    k
}

/// The digest of `data` in hexadecimal.
pub fn hex_digest(data: &[u8]) -> String {
    let k = table();
    let mut msg = data.to_vec();
    let bits = (data.len() as u64).wrapping_mul(8);
    msg.push(0x80);
    while msg.len() % 64 != 56 {
        msg.push(0);
    }
    msg.extend_from_slice(&bits.to_le_bytes());
    let (mut a0, mut b0, mut c0, mut d0) = (0x67452301u32, 0xefcdab89u32, 0x98badcfeu32, 0x10325476u32);
    for chunk in msg.as_chunks::<64>().0 {
        let mut m = [0u32; 16];
        for (slot, w) in m.iter_mut().zip(chunk.as_chunks::<4>().0) {
            *slot = u32::from_le_bytes(*w);
        }
        let (mut a, mut b, mut c, mut d) = (a0, b0, c0, d0);
        for i in 0..64 {
            let (f, g) = match i / 16 {
                0 => ((b & c) | (!b & d), i),
                1 => ((d & b) | (!d & c), (5 * i + 1) % 16),
                2 => (b ^ c ^ d, (3 * i + 5) % 16),
                _ => (c ^ (b | !d), (7 * i) % 16),
            };
            let f = f
                .wrapping_add(a)
                .wrapping_add(k.get(i).copied().unwrap_or(0))
                .wrapping_add(m.get(g).copied().unwrap_or(0));
            a = d;
            d = c;
            c = b;
            b = b.wrapping_add(f.rotate_left(S.get(i).copied().unwrap_or(0)));
        }
        a0 = a0.wrapping_add(a);
        b0 = b0.wrapping_add(b);
        c0 = c0.wrapping_add(c);
        d0 = d0.wrapping_add(d);
    }
    [a0, b0, c0, d0].iter().flat_map(|v| v.to_le_bytes()).map(|b| format!("{b:02x}")).collect()
}

#[cfg(test)]
mod tests {
    #[test]
    fn rfc_1321_test_suite() {
        assert_eq!(super::hex_digest(b""), "d41d8cd98f00b204e9800998ecf8427e");
        assert_eq!(super::hex_digest(b"abc"), "900150983cd24fb0d6963f7d28e17f72");
        assert_eq!(super::hex_digest(b"message digest"), "f96b697d7cb7938d525a2f31aaf161d0");
        assert_eq!(
            super::hex_digest(
                b"12345678901234567890123456789012345678901234567890123456789012345678901234567890"
            ),
            "57edf4a22be3c955ac49da2e2107b67a"
        );
    }
}
