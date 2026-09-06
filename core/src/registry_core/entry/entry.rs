//! Public development entry for applications embedding NichLink.
//! 供嵌入 NichLink 的应用使用的公开开发入口。

/// Open NichLink Studio when this package is used from a debug build.
/// 在调试构建中打开 NichLink Studio。
///
/// Release builds return immediately and contain no Ratatui code. The Studio
/// process is located beside this package, so a frontend can call this same
/// entry without maintaining a second launch command.
/// 发布构建立即返回且不包含 Ratatui 代码。Studio 位于本包旁边，前端无需
/// 维护第二套启动命令即可调用同一个入口。
pub fn run() -> Result<(), String> {
    run_with_query(None)
}

/// Open Studio and optionally seed its search overlay.
/// 打开 Studio，可选地预填搜索悬浮窗。
pub fn run_with_query(query: Option<&str>) -> Result<(), String> {
    #[cfg(debug_assertions)]
    {
        let _ = query;
        Err("NichLink Studio is a separate crate; launch it from its own package".to_owned())
    }
    #[cfg(not(debug_assertions))]
    {
        let _ = query;
        Ok(())
    }
}
