/// Compute SHA-256 over two slices, optionally separated by a zero byte.
/// 对两个字节片段计算 SHA-256，可选地在中间加入零字节。
/// One hashed byte, with path separators folded when this hash identifies a
/// path. `file!()` records the platform separator, so folding here keeps a
/// face's identity the same on every platform without allocating.
/// 参与哈希的一个字节；当该哈希用于路径身份时折叠分隔符。`file!()` 记录的是
/// 平台分隔符，在这里折叠就能在不分配的前提下让注册面的身份跨平台一致。
const fn path_byte(value: u8, normalize_path: bool) -> u8 {
    if normalize_path && value == b'\\' {
        b'/'
    } else {
        value
    }
}

const fn sha256_digest(
    first: &[u8],
    second: &[u8],
    separator: bool,
    normalize_path: bool,
) -> [u8; 32] {
    const K: [u32; 64] = [
        0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4,
        0xab1c5ed5, 0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe,
        0x9bdc06a7, 0xc19bf174, 0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f,
        0x4a7484aa, 0x5cb0a9dc, 0x76f988da, 0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7,
        0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967, 0x27b70a85, 0x2e1b2138, 0x4d2c6dfc,
        0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85, 0xa2bfe8a1, 0xa81a664b,
        0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070, 0x19a4c116,
        0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
        0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7,
        0xc67178f2,
    ];
    let separator_len = if separator { 1 } else { 0 };
    let length = first.len() + separator_len + second.len();
    let blocks = (length + 9).div_ceil(64);
    let padded = blocks * 64;
    let bits = (length as u64) * 8;
    let mut state = [
        0x6a09e667_u32,
        0xbb67ae85,
        0x3c6ef372,
        0xa54ff53a,
        0x510e527f,
        0x9b05688c,
        0x1f83d9ab,
        0x5be0cd19,
    ];
    let mut block = 0;
    while block < blocks {
        let mut words = [0_u32; 64];
        let mut i = 0;
        while i < 16 {
            let mut value = 0_u32;
            let mut byte = 0;
            while byte < 4 {
                let offset = block * 64 + i * 4 + byte;
                let input = if offset < length {
                    if offset < first.len() {
                        path_byte(first[offset], normalize_path)
                    } else if separator && offset == first.len() {
                        0
                    } else {
                        path_byte(second[offset - first.len() - separator_len], normalize_path)
                    }
                } else if offset == length {
                    0x80
                } else if offset >= padded - 8 {
                    ((bits >> ((padded - 1 - offset) * 8)) & 0xff) as u8
                } else {
                    0
                };
                value |= (input as u32) << (24 - byte * 8);
                byte += 1;
            }
            words[i] = value;
            i += 1;
        }
        while i < 64 {
            let a = words[i - 15].rotate_right(7)
                ^ words[i - 15].rotate_right(18)
                ^ (words[i - 15] >> 3);
            let b = words[i - 2].rotate_right(17)
                ^ words[i - 2].rotate_right(19)
                ^ (words[i - 2] >> 10);
            words[i] = words[i - 16]
                .wrapping_add(a)
                .wrapping_add(words[i - 7])
                .wrapping_add(b);
            i += 1;
        }
        let (mut a, mut b, mut c, mut d, mut e, mut f, mut g, mut h) = (
            state[0], state[1], state[2], state[3], state[4], state[5], state[6], state[7],
        );
        i = 0;
        while i < 64 {
            let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
            let ch = (e & f) ^ ((!e) & g);
            let t1 = h
                .wrapping_add(s1)
                .wrapping_add(ch)
                .wrapping_add(K[i])
                .wrapping_add(words[i]);
            let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
            let maj = (a & b) ^ (a & c) ^ (b & c);
            let t2 = s0.wrapping_add(maj);
            h = g;
            g = f;
            f = e;
            e = d.wrapping_add(t1);
            d = c;
            c = b;
            b = a;
            a = t1.wrapping_add(t2);
            i += 1;
        }
        state[0] = state[0].wrapping_add(a);
        state[1] = state[1].wrapping_add(b);
        state[2] = state[2].wrapping_add(c);
        state[3] = state[3].wrapping_add(d);
        state[4] = state[4].wrapping_add(e);
        state[5] = state[5].wrapping_add(f);
        state[6] = state[6].wrapping_add(g);
        state[7] = state[7].wrapping_add(h);
        block += 1;
    }
    let mut output = [0_u8; 32];
    let mut i = 0;
    while i < 8 {
        let bytes = state[i].to_be_bytes();
        output[i * 4] = bytes[0];
        output[i * 4 + 1] = bytes[1];
        output[i * 4 + 2] = bytes[2];
        output[i * 4 + 3] = bytes[3];
        i += 1;
    }
    output
}

/// Compute SHA-256 over two slices, optionally separated by a zero byte, and
/// keep the first 128 bits.
/// 对两个字节片段计算 SHA-256，可选地在中间加入零字节，并取前 128 位。
pub(super) const fn sha256_prefix(first: &[u8], second: &[u8], separator: bool) -> [u8; 16] {
    let digest = sha256_digest(first, second, separator, false);
    sha256_prefix_of(digest)
}

/// Like [`sha256_prefix`], but `/` and `\` hash to the same byte so a path's
/// identity does not depend on the platform separator.
/// 与 [`sha256_prefix`] 相同，但 `/` 与 `\` 哈希为同一字节，路径身份因此不依赖
/// 平台分隔符。
pub(super) const fn sha256_path_prefix(first: &[u8], second: &[u8], separator: bool) -> [u8; 16] {
    let digest = sha256_digest(first, second, separator, true);
    sha256_prefix_of(digest)
}

const fn sha256_prefix_of(digest: [u8; 32]) -> [u8; 16] {
    let mut output = [0_u8; 16];
    let mut i = 0;
    while i < 16 {
        output[i] = digest[i];
        i += 1;
    }
    output
}

/// Dependency-free SHA-256, hex-encoded. Shares the digest core with
/// `sha256_prefix`; do not add a second implementation.
/// 零依赖 SHA-256，输出十六进制。与 `sha256_prefix` 共用同一个摘要核心，
/// 不要再写第二份实现。
///
/// Public because the plugin host checks a plugin's checksum and key
/// fingerprint against it; those bytes never pass through a `NodeId`.
/// 公开是因为插件宿主用它校验插件校验和与公钥指纹；那些字节从不经过 `NodeId`。
pub fn sha256_hex(input: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let digest = sha256_digest(input, &[], false, false);
    let mut out = String::with_capacity(64);
    for byte in digest {
        out.push(HEX[(byte >> 4) as usize] as char);
        out.push(HEX[(byte & 0x0f) as usize] as char);
    }
    out
}

/// Whether a byte is a path separator on either supported platform.
/// 该字节是否是两个受支持平台上的路径分隔符。
pub(super) const fn is_separator(byte: u8) -> bool {
    byte == b'/' || byte == b'\\'
}

/// Drop one leading path separator, when present.
/// 存在时去掉一个前导路径分隔符。
pub(super) const fn strip_leading_separator(value: &str) -> &str {
    let bytes = value.as_bytes();
    if bytes.is_empty() || !is_separator(bytes[0]) {
        return value;
    }
    let (_, rest) = bytes.split_at(1);
    match core::str::from_utf8(rest) {
        Ok(text) => text,
        Err(_) => value,
    }
}

#[cfg(test)]
mod tests {
    use super::{sha256_hex, sha256_path_prefix, sha256_prefix};

    /// The independent oracle: the audited `sha2` crate, reached only through
    /// the dev-dependency, so the shipped kernel stays dependency-free.
    /// 独立预言机：受审计的 `sha2` crate，仅经 dev-dependency 引入，因此发布的内核
    /// 保持零依赖。
    fn oracle(input: &[u8]) -> [u8; 32] {
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(input);
        hasher.finalize().into()
    }

    /// Lowercase hex of a byte slice, written here instead of reusing the
    /// implementation's encoder so a shared encoder bug cannot cancel out.
    /// 字节切片的小写十六进制；这里另写一份而不用实现自身的编码器，避免共用的编码
    /// 错误相互抵消。
    fn hex(bytes: &[u8]) -> String {
        const DIGITS: &[u8; 16] = b"0123456789abcdef";
        let mut out = String::with_capacity(bytes.len() * 2);
        for byte in bytes {
            out.push(DIGITS[(byte >> 4) as usize] as char);
            out.push(DIGITS[(byte & 0x0f) as usize] as char);
        }
        out
    }

    /// Deterministic filler, so a failure is reproducible from its length alone.
    /// 确定性填充字节，因此仅凭长度即可复现失败。
    fn filler(length: usize) -> Vec<u8> {
        (0..length)
            .map(|index| (index as u8).wrapping_mul(31).wrapping_add(7))
            .collect()
    }

    /// Differential proof across the whole padding surface. The obvious
    /// evidence — the two published vectors — is wrong: both fit in one block,
    /// so neither exercises the second block, and the hand-written loop shares
    /// one block path between single- and two-slice inputs. The boundary is
    /// every length in `0..=200`, which straddles the 55/56-byte point where
    /// padding spills into a second block and the 119/120-byte point where the
    /// 64-bit length field itself needs a second block, plus a multi-block and
    /// a 1 MiB input. `sha2` is the oracle; this test is the pin.
    /// 覆盖整个填充面的差分证明。显而易见的证据——两个公开向量——是错的：它们都只
    /// 占一个分组，谁也走不到第二个分组，而手写循环让单片段与双片段输入共用同一条
    /// 分组路径。边界是 `0..=200` 的全部长度——它跨越 55/56 字节处（填充溢出到第二
    /// 个分组）与 119/120 字节处（64 位长度字段本身需要第二个分组）——外加一个多分组
    /// 输入和一个 1 MiB 输入。`sha2` 是预言机，本测试就是钉。
    #[test]
    fn matches_sha2_across_lengths_and_padding_boundaries() {
        let mut lengths: Vec<usize> = (0..=200).collect();
        // Named explicitly so the boundaries are visible to a reader even though
        // `0..=200` already contains the smaller ones.
        // 显式列出，让边界对读者可见，尽管其中较小的那些已包含在 `0..=200` 内。
        lengths.extend([55, 56, 57, 63, 64, 65, 119, 120, 127, 128, 1_000, 1_048_576]);
        for length in lengths {
            let input = filler(length);
            assert_eq!(
                sha256_hex(&input),
                hex(&oracle(&input)),
                "single-slice digest differed at length {length}"
            );
        }
    }

    /// The two-slice + separator composition is what node identity actually
    /// uses, and nothing pins it by value. The obvious reading is wrong in two
    /// ways: `separator = false` must still concatenate with no byte in
    /// between, and the separator is one `0x00`, not the path separator. The
    /// boundary is the `separator` branch inside `sha256_digest`; this test is
    /// the pin, checking `first || 0x00 || second` and `first || second`.
    /// 双片段 + 分隔符组合正是节点身份实际使用的形式，而没有任何测试按值钉住它。
    /// 显而易见的读法在两点上是错的：`separator = false` 仍必须直接拼接、中间不插
    /// 字节，且分隔符是一个 `0x00` 而不是路径分隔符。边界是 `sha256_digest` 内部的
    /// `separator` 分支；本测试就是钉，校验 `first || 0x00 || second` 与
    /// `first || second`。
    #[test]
    fn prefix_matches_sha2_over_the_two_slice_composition() {
        let pairs: [(&[u8], &[u8]); 5] = [
            (b"control/control.rs", b"Control"),
            (b"app", b"control/control.rs"),
            (b"", b"root"),
            (b"<root>", b"root"),
            (b"a", b"bc"),
        ];
        for (first, second) in pairs {
            let mut separated = first.to_vec();
            separated.push(0);
            separated.extend_from_slice(second);
            let mut joined = first.to_vec();
            joined.extend_from_slice(second);

            assert_eq!(
                sha256_prefix(first, second, true),
                oracle(&separated)[..16],
                "separated composition differed for {first:?} / {second:?}"
            );
            assert_eq!(
                sha256_prefix(first, second, false),
                oracle(&joined)[..16],
                "unseparated composition differed for {first:?} / {second:?}"
            );
        }
    }

    /// Folding must rewrite `\` to `/` on BOTH slices, because
    /// `from_namespaced_path` passes the namespace first and the path second.
    /// The obvious implementation — folding only the path slice — still passes
    /// every existing relational test, so this value-level check is the pin.
    /// The boundary is the pair whose backslash sits in the second slice, next
    /// to one where it sits in the first.
    /// 折叠必须把两个片段上的 `\` 都改写为 `/`，因为 `from_namespaced_path` 把命名
    /// 空间放在第一片段、路径放在第二片段。显而易见的实现——只折叠路径片段——仍能
    /// 通过现有全部关系型测试，因此这个按值校验才是钉。边界是反斜杠位于第二片段的那
    /// 一对，以及反斜杠位于第一片段的另一对作为对照。
    #[test]
    fn path_prefix_matches_sha2_over_folded_two_slice_composition() {
        let pairs: [(&[u8], &[u8]); 3] = [
            (b"app", b"control\\control.rs"),
            (b"a\\b", b"c\\d"),
            (b"control/control.rs", b"Control"),
        ];
        for (first, second) in pairs {
            let mut folded_first = first.to_vec();
            for byte in &mut folded_first {
                if *byte == b'\\' {
                    *byte = b'/';
                }
            }
            let mut folded_second = second.to_vec();
            for byte in &mut folded_second {
                if *byte == b'\\' {
                    *byte = b'/';
                }
            }
            let mut folded = folded_first;
            folded.push(0);
            folded.extend_from_slice(&folded_second);

            assert_eq!(
                sha256_path_prefix(first, second, true),
                oracle(&folded)[..16],
                "folded composition differed for {first:?} / {second:?}"
            );
        }
    }
}
