//! Dependency-free, compile-time node identities.
//! 零依赖、编译期计算的节点身份。
//!
//! Contract 契约:
//! - The same `(namespace, relative_path, declared_name)` triple always
//!   hashes to the same 16 bytes, on every tool and every run. This is what
//!   lets the build surface, runtime, plugin locks, and caches agree on an
//!   identity without coordinating.
//!   同样的三元组在任何工具、任何运行中永远算出同样的 16 字节——构建面、
//!   运行时、插件锁和缓存因此无需协调即可认同一个身份。
//! - NodeId is a 128-bit prefix of SHA-256 for indexing and diagnostics,
//!   never a cryptographic signature. Plugin trust uses the full digest and
//!   a signature verifier.
//!   NodeId 是用于索引和诊断的 128 位截断，不是密码学签名。插件信任走
//!   完整摘要和签名验证器。
//! - `IDENTITY_SCHEMA` versions the hashing input format; bump it in the same
//!   commit that changes any serialization feeding the hash. Persisted
//!   identity-adjacent formats (identity cache, plugin lock) carry parallel
//!   schema headers of their own.
//!   `IDENTITY_SCHEMA` 给哈希输入格式定版本；改动任何喂给哈希的序列化时，
//!   在同一个 commit 里 +1。持久化格式（身份缓存、插件锁）带有各自的
//!   并行 schema 头。
//!
//! Sections: `node_id`, `stable_face_id`, hex helpers, and the private
//! `sha256` core. One type per file, order follows the reader's questions.
//! 分区：`node_id`、`stable_face_id`、hex 辅助和私有 `sha256` 核心。
//! 一个类型一个文件，顺序遵循读者的提问次序。

mod hex;
mod node_id;
mod parse_error;
mod sha256;
mod stable_face_id;

pub use hex::{hex_decode, hex_nibble};
pub use node_id::{NodeId, ROOT_NODE_ID, root_node_id};
pub use parse_error::ParseNodeIdError;
pub use sha256::sha256_hex;
pub use stable_face_id::StableFaceId;

/// Version of the identity input format.
/// 身份输入格式的版本。
pub const IDENTITY_SCHEMA: &str = "3";
