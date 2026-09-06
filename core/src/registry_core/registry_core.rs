//! The registry-core package contains no concrete object implementation.
//! registry_core 包不包含任何具体 object 实现。
//!
//! Its sibling files each own one part of the registration mechanism. The
//! build script discovers and re-exports them from this package automatically.
//! 同级文件各自负责注册机制的一部分，并由构建脚本自动发现和导出。

// Registry never exposes the collection backend as part of its data model.
// Registry 不把收集后端暴露为数据模型的一部分。
