//! A logical face identity that can survive a source-file move.
//! 可跨源码文件移动保持不变的逻辑注册面身份。
//!
//! `NodeId` is the source-instance identity used by the tree and pruning.
//! `StableFaceId` is opt-in and survives a source-file move.
//! `NodeId` 是注册树和修剪使用的源码实例身份；`StableFaceId` 可跨文件移动保持不变。

use std::fmt;

use super::sha256::sha256_prefix;

/// A logical face identity that can survive a source-file move.
/// 可跨源码文件移动保持不变的逻辑注册面身份。
///
/// `NodeId` is the source-instance identity used by the tree and pruning.
/// `StableFaceId` is opt-in and survives a source-file move.
/// `NodeId` 是注册树和修剪使用的源码实例身份；`StableFaceId` 可跨文件移动保持不变。
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct StableFaceId([u8; 16]);

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
