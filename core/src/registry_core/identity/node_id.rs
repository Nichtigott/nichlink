//! Stable source identity shared by registration, diagnostics, and pruning.
//! 注册、诊断和修剪共用的稳定源码身份。
//!
//! This is a compact 128-bit prefix of SHA-256 for indexing and diagnostics,
//! not a cryptographic signature. Plugin trust must use the full digest and a
//! signature verifier.
//! 这是用于索引和诊断的 SHA-256 128 位短标识，不是密码学签名。插件信任必须
//! 使用完整摘要和签名验证器。

use std::fmt;
use std::str::FromStr;

use super::hex::hex_nibble;
use super::parse_error::ParseNodeIdError;
use super::sha256::sha256_prefix;

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

/// The only node not registered through an object declaration.
/// 唯一不通过对象声明注册的节点。
pub const ROOT_NODE_ID: NodeId = NodeId::from_path("<root>", "root");

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

    /// Hash a declaration together with the package namespace that owns it.
    /// 将声明与拥有它的包命名空间一起散列，避免不同库的相对路径串库。
    pub const fn from_namespaced_path(
        namespace: &str,
        relative_path: &str,
        declared_name: &str,
    ) -> Self {
        // Hashing the first two components and feeding that digest into the
        // final hash keeps the implementation const and allocation-free while
        // making the namespace part of the identity domain.
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

/// Return the root identity for one isolated package namespace.
pub const fn root_node_id(namespace: &str) -> NodeId {
    NodeId::from_namespaced_path(namespace, "<root>", "root")
}

impl fmt::Display for NodeId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        for byte in self.0 {
            write!(formatter, "{byte:02x}")?;
        }
        Ok(())
    }
}

impl fmt::Debug for NodeId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, formatter)
    }
}

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

#[cfg(test)]
mod tests {
    use super::{NodeId, ROOT_NODE_ID, root_node_id};

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
            ),
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
    fn namespace_changes_node_identity() {
        assert_ne!(
            NodeId::from_namespaced_path("library-a", "src/button.rs", "Button"),
            NodeId::from_namespaced_path("library-b", "src/button.rs", "Button")
        );
    }

    #[test]
    fn namespaced_roots_are_distinct_and_legacy_root_remains_stable() {
        assert_ne!(root_node_id("library-a"), root_node_id("library-b"));
        assert_ne!(ROOT_NODE_ID, root_node_id("library-a"));
    }
}
