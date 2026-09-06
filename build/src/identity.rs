//! Dependency-free, compile-time SHA-256 node identities.
//! 零依赖、编译期计算的 SHA-256 节点身份。

use std::fmt;
use std::str::FromStr;

/// Version of the identity input and persisted catalog formats.
/// 身份输入与持久化目录格式的版本。
pub const IDENTITY_SCHEMA: &str = "3";

/// Stable source identity shared by registration, diagnostics, and pruning.
/// 注册、诊断和修剪共用的稳定源码身份。
///
/// This is a compact 128-bit prefix of SHA-256 for indexing and diagnostics,
/// not a cryptographic signature. Plugin trust must use the full digest and a
/// signature verifier.
/// 这是用于索引和诊断的 SHA-256 128 位短标识，不是密码学签名。插件信任必须
/// 使用完整摘要和签名验证器。
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NodeId([u8; 16]);

/// A logical face identity that can survive a source-file move.
/// 可跨源码文件移动保持不变的逻辑注册面身份。
///
/// `NodeId` is the source-instance identity used by the tree and pruning.
/// `StableFaceId` is opt-in and survives a source-file move.
/// `NodeId` 是注册树和修剪使用的源码实例身份；`StableFaceId` 可跨文件移动保持不变。
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct StableFaceId([u8; 16]);

/// The only node not registered through an object declaration.
/// 唯一不通过对象声明注册的节点。
pub const ROOT_NODE_ID: NodeId = NodeId::from_path("<root>", "root");

/// Return the namespace of the package whose build script is currently running.
///
/// Cargo exposes the consuming package name to a build-script process. Using
/// that value keeps build-time identities byte-for-byte compatible with the
/// `env!("CARGO_PKG_NAME")` value captured by the declaration macros.
pub fn package_namespace() -> String {
    std::env::var("CARGO_PKG_NAME").unwrap_or_else(|_| "nichlink.default".to_owned())
}

/// Compute the identity used by generated plans and caches.
pub fn package_node_id(relative_path: &str, declared_name: &str) -> NodeId {
    let namespace = package_namespace();
    NodeId::from_namespaced_path(&namespace, relative_path, declared_name)
}

/// Compute the root identity used by generated plans and caches.
pub fn package_root_node_id() -> NodeId {
    let namespace = package_namespace();
    NodeId::from_namespaced_path(&namespace, "<root>", "root")
}

impl NodeId {
    /// Rebuild an identity emitted by NichLink's build step.
    /// 从 NichLink 构建步骤生成的字节恢复身份。
    #[doc(hidden)]
    pub const fn from_raw(bytes: [u8; 16]) -> Self {
        Self(bytes)
    }

    /// Hash `relative_path + 0x00 + declared_name` during constant evaluation.
    /// 在常量求值期间散列“相对路径 + 0x00 + 声明名”。
    pub const fn from_path(relative_path: &str, declared_name: &str) -> Self {
        Self(sha256_prefix(
            relative_path.as_bytes(),
            declared_name.as_bytes(),
            true,
        ))
    }

    /// Hash a declaration together with its owning package namespace.
    pub const fn from_namespaced_path(
        namespace: &str,
        relative_path: &str,
        declared_name: &str,
    ) -> Self {
        let scoped = sha256_prefix(namespace.as_bytes(), relative_path.as_bytes(), true);
        Self(sha256_prefix(&scoped, declared_name.as_bytes(), true))
    }

    /// Hash one byte slice; primarily useful for standard-vector tests.
    /// 散列单个字节切片，主要用于标准向量测试。
    pub const fn from_bytes(input: &[u8]) -> Self {
        Self(sha256_prefix(input, &[], false))
    }

    pub const fn into_bytes(self) -> [u8; 16] {
        self.0
    }

    /// Produce a node-specific compile-time payload for pruning verification.
    /// 为修剪验证生成节点专属的编译期负载。
    ///
    /// The table is emitted only when the owning node's probe is reached.
    /// 只有触达所属节点探针时，这张表才会进入最终产物。
    pub const fn pruning_table(self) -> [u64; 4096] {
        let bytes = self.0;
        let mut state = u64::from_le_bytes([
            bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7],
        ]);
        let mut table = [0_u64; 4096];
        let mut index = 0;
        while index < table.len() {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            table[index] = state.wrapping_add(index as u64);
            index += 1;
        }
        table
    }
}

impl StableFaceId {
    /// Hash an author-owned logical name, independent of its source path.
    /// 对作者拥有的逻辑名称做哈希，与源码路径无关。
    pub const fn from_name(name: &str) -> Self {
        Self(sha256_prefix(name.as_bytes(), &[], false))
    }

    pub const fn into_bytes(self) -> [u8; 16] {
        self.0
    }
}

impl fmt::Display for NodeId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        for byte in self.0 {
            write!(formatter, "{byte:02x}")?;
        }
        Ok(())
    }
}

impl fmt::Display for StableFaceId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        for byte in self.0 {
            write!(formatter, "{byte:02x}")?;
        }
        Ok(())
    }
}

impl fmt::Debug for StableFaceId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, formatter)
    }
}

impl fmt::Debug for NodeId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, formatter)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ParseNodeIdError;

impl fmt::Display for ParseNodeIdError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("node identity must contain exactly 32 hexadecimal digits")
    }
}

impl std::error::Error for ParseNodeIdError {}

impl FromStr for NodeId {
    type Err = ParseNodeIdError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        if value.len() != 32 {
            return Err(ParseNodeIdError);
        }
        let bytes = value.as_bytes();
        let mut id = [0_u8; 16];
        for index in 0..16 {
            let high = hex_nibble(bytes[index * 2]).ok_or(ParseNodeIdError)?;
            let low = hex_nibble(bytes[index * 2 + 1]).ok_or(ParseNodeIdError)?;
            id[index] = (high << 4) | low;
        }
        Ok(Self(id))
    }
}

fn hex_nibble(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

/// Compute SHA-256 over two slices, optionally separated by a zero byte, and
/// keep the first 128 bits.
/// 对两个字节片段计算 SHA-256，可选地在中间加入零字节，并取前 128 位。
const fn sha256_prefix(first: &[u8], second: &[u8], separator: bool) -> [u8; 16] {
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
                        first[offset]
                    } else if separator && offset == first.len() {
                        0
                    } else {
                        second[offset - first.len() - separator_len]
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
    let mut output = [0_u8; 16];
    let mut i = 0;
    while i < 4 {
        let bytes = state[i].to_be_bytes();
        output[i * 4] = bytes[0];
        output[i * 4 + 1] = bytes[1];
        output[i * 4 + 2] = bytes[2];
        output[i * 4 + 3] = bytes[3];
        i += 1;
    }
    output
}

/// Small dependency-free SHA-256 implementation used for plugin verification.
/// 用于插件校验的零依赖 SHA-256 实现。
pub fn sha256_hex(input: &[u8]) -> String {
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
    let mut data = input.to_vec();
    let bits = (data.len() as u64) * 8;
    data.push(0x80);
    while data.len() % 64 != 56 {
        data.push(0);
    }
    data.extend_from_slice(&bits.to_be_bytes());
    let mut h = [
        0x6a09e667u32,
        0xbb67ae85,
        0x3c6ef372,
        0xa54ff53a,
        0x510e527f,
        0x9b05688c,
        0x1f83d9ab,
        0x5be0cd19,
    ];
    for chunk in data.chunks_exact(64) {
        let mut w = [0u32; 64];
        for i in 0..16 {
            w[i] = u32::from_be_bytes([
                chunk[i * 4],
                chunk[i * 4 + 1],
                chunk[i * 4 + 2],
                chunk[i * 4 + 3],
            ]);
        }
        for i in 16..64 {
            let s0 = w[i - 15].rotate_right(7) ^ w[i - 15].rotate_right(18) ^ (w[i - 15] >> 3);
            let s1 = w[i - 2].rotate_right(17) ^ w[i - 2].rotate_right(19) ^ (w[i - 2] >> 10);
            w[i] = w[i - 16]
                .wrapping_add(s0)
                .wrapping_add(w[i - 7])
                .wrapping_add(s1);
        }
        let (mut a, mut b, mut c, mut d, mut e, mut f, mut g, mut hh) =
            (h[0], h[1], h[2], h[3], h[4], h[5], h[6], h[7]);
        for i in 0..64 {
            let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
            let ch = (e & f) ^ ((!e) & g);
            let t1 = hh
                .wrapping_add(s1)
                .wrapping_add(ch)
                .wrapping_add(K[i])
                .wrapping_add(w[i]);
            let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
            let maj = (a & b) ^ (a & c) ^ (b & c);
            let t2 = s0.wrapping_add(maj);
            hh = g;
            g = f;
            f = e;
            e = d.wrapping_add(t1);
            d = c;
            c = b;
            b = a;
            a = t1.wrapping_add(t2);
        }
        h[0] = h[0].wrapping_add(a);
        h[1] = h[1].wrapping_add(b);
        h[2] = h[2].wrapping_add(c);
        h[3] = h[3].wrapping_add(d);
        h[4] = h[4].wrapping_add(e);
        h[5] = h[5].wrapping_add(f);
        h[6] = h[6].wrapping_add(g);
        h[7] = h[7].wrapping_add(hh);
    }
    let mut out = String::with_capacity(64);
    for word in h {
        use std::fmt::Write as _;
        write!(out, "{word:08x}").unwrap();
    }
    out
}

#[cfg(test)]
mod tests {
    use super::{package_namespace, package_node_id, sha256_hex, NodeId};

    const ABC: NodeId = NodeId::from_bytes(b"abc");
    const MULTI_BLOCK: NodeId =
        NodeId::from_bytes(b"abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq");

    #[test]
    fn matches_the_standard_vector() {
        assert_eq!(ABC.to_string(), "ba7816bf8f01cfea414140de5dae2223");
        assert_eq!(ABC.to_string().parse(), Ok(ABC));
        assert_eq!(MULTI_BLOCK.to_string(), "248d6a61d20638b8e5c026930c3e6039");
    }

    #[test]
    fn path_is_part_of_the_identity() {
        assert_ne!(
            NodeId::from_path("control/object/button/button.rs", "Button"),
            NodeId::from_path(
                "engine/role/style/object/composite_style/object/button/button.rs",
                "Button",
            )
        );
    }

    #[test]
    fn path_components_are_unambiguous() {
        assert_ne!(
            NodeId::from_path("ab", "c"),
            NodeId::from_path("a", "bc"),
            "the path/name separator must be part of the identity input"
        );
    }

    #[test]
    fn sha256_matches_standard_vector() {
        assert_eq!(
            sha256_hex(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn package_identity_matches_the_macro_namespace_algorithm() {
        let namespace = package_namespace();
        let path = "control/object/button/button.rs";
        let kind = "Button";
        assert_eq!(
            package_node_id(path, kind),
            NodeId::from_namespaced_path(&namespace, path, kind)
        );
        assert_ne!(
            NodeId::from_namespaced_path("library-a", path, kind),
            NodeId::from_namespaced_path("library-b", path, kind)
        );
    }
}
