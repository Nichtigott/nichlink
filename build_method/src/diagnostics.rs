//! Build-phase diagnostic model. The types live in the kernel; this module
//! keeps the historical `crate::diagnostics` import paths.
//! 构建期诊断模型。类型本体在 kernel；本模块保留 `crate::diagnostics` 历史导入路径。

pub(crate) use nichlink::{BuildDiagnostic, BuildDiagnostics};
