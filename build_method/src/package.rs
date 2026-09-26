//! The package identity Cargo reports for a directory.
//! Cargo 为某个目录报告的包身份。
//!
//! A host's package name *is* its `NodeId` namespace: every face identity is a
//! hash over `(namespace, source path, declared name)`, and the declaration
//! macros bake `env!("CARGO_PKG_NAME")` in as that namespace. Two surfaces need
//! the same answer — `nichlink-cli` to name faces from the command line, and the
//! MCP bridge to report the tree it is asked about — so the read lives in one
//! place and the policy is stated once: Cargo answers, and a question Cargo
//! cannot answer is an error rather than a guess.
//! 宿主的包名**就是**它的 `NodeId` 命名空间：每个面的身份都是对
//! `(命名空间, 源码路径, 声明名)` 的散列，而声明宏把 `env!("CARGO_PKG_NAME")` 烤进去作为
//! 该命名空间。两个执行面需要同一个答案——`nichlink-cli` 从命令行命名面，MCP 桥报告被问到的
//! 那棵树——因此这次读取只有一个住址，策略也只声明一次：由 Cargo 作答，而 Cargo 答不出的问题
//! 是错误，不是猜测。
//!
//! Nothing in the build pipeline calls this: a build script receives
//! `CARGO_PKG_NAME` from Cargo, which is the same value, so the subprocess below
//! happens only when a standalone surface asks. That distinction is why this
//! stays a plain function instead of being wired into `pipeline::run`.
//! 构建管线不调用这里：构建脚本从 Cargo 拿到 `CARGO_PKG_NAME`，也就是同一个值，因此下面的
//! 子进程只会在独立执行面提问时发生。正因如此，它是普通函数，而不接进 `pipeline::run`。

use std::path::Path;

/// Ask Cargo for the package name of the package whose manifest is `manifest`.
/// 向 Cargo 询问清单为 `manifest` 的那个包的包名。
///
/// The parameter is the manifest **file**, not its directory: that is the thing
/// the caller actually has — a surface that resolved a project holds a manifest
/// path, possibly one an environment variable named — and it is also what Cargo
/// itself takes. A directory would bake in the assumption that the manifest is
/// called `Cargo.toml`; Cargo refuses any other name anyway ("the manifest-path
/// must be a path to a Cargo.toml file"), so the assumption is not even useful.
/// 参数是清单**文件**而不是它所在目录：那正是调用方实际持有的东西——解析了项目的执行面拿到的是一条
/// 清单路径，还可能来自环境变量——而且这也是 Cargo 自己接受的取值。传目录会把"清单就叫
/// `Cargo.toml`"这个假设烤进接口；Cargo 反正拒绝别的名字（"the manifest-path must be a path to a
/// Cargo.toml file"），所以这个假设连用处都没有。
///
/// Reading `[package] name` by hand was wrong three ways at once: it did not
/// know TOML sections, so a `[lib]` or `[[bin]]` name key could answer first; it
/// did not know single-quoted strings; and when it found nothing it silently
/// returned the directory name. The package name is the `NodeId` namespace, so a
/// wrong answer makes the same crate report one identity under `nichlink check`
/// and another under a real build. Cargo is the authority, and a question it
/// cannot answer is an error rather than a guess.
/// 手写读 `[package] name` 同时错了三处：它不认识 TOML 区段，于是 `[lib]` 或
/// `[[bin]]` 的 name 键可能抢先作答；它不认识单引号字符串；找不到时还会静默返回目录
/// 名。包名是 `NodeId` 的命名空间，答错就会让同一个 crate 在 `nichlink check` 与真实
/// 编译下报告两种身份。Cargo 才是权威；它答不出的问题一律报错，而不是猜。
pub fn package_name(manifest: &Path) -> Result<String, String> {
    let output = std::process::Command::new("cargo")
        .args(["metadata", "--format-version", "1", "--no-deps"])
        .arg("--manifest-path")
        .arg(manifest)
        .output()
        .map_err(|error| format!("cannot run cargo metadata: {error}"))?;
    if !output.status.success() {
        return Err(format!(
            "cargo metadata failed for {}: {}",
            manifest.display(),
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    let metadata: serde_json::Value = serde_json::from_slice(&output.stdout)
        .map_err(|error| format!("cannot read cargo metadata output: {error}"))?;
    let packages = metadata["packages"]
        .as_array()
        .ok_or_else(|| "cargo metadata reported no packages".to_owned())?;
    // `--no-deps` lists the whole workspace, so the package has to be the one
    // whose manifest this is; taking the first entry would answer for a sibling
    // member.
    // `--no-deps` 会列出整个 workspace，因此必须挑出 manifest 正是这一个的包；取第一
    // 项会替同工作区的另一个成员作答。
    let package = packages
        .iter()
        .find(|package| {
            package["manifest_path"].as_str().is_some_and(|path| {
                Path::new(path).parent().is_some_and(|parent| {
                    same_directory(parent, manifest.parent().unwrap_or_else(|| Path::new(".")))
                })
            })
        })
        .ok_or_else(|| {
            format!(
                "{} is not a package; cargo metadata listed {} workspace member(s)",
                manifest.display(),
                packages.len()
            )
        })?;
    package["name"].as_str().map(str::to_owned).ok_or_else(|| {
        format!(
            "cargo metadata reported no package name for {}",
            manifest.display()
        )
    })
}

/// Whether two paths name the same directory, following symlinks when possible.
/// 两个路径是否指向同一目录，能跟随符号链接时跟随。
fn same_directory(left: &Path, right: &Path) -> bool {
    let canonical =
        |path: &Path| std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
    canonical(left) == canonical(right)
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::package_name;

    /// A throwaway directory, unique per call.
    /// 每次调用唯一的临时目录。
    fn temporary_root(label: &str) -> PathBuf {
        static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let sequence = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let root = std::env::temp_dir().join(format!(
            "nichlink-package-{label}-{}-{sequence}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).expect("fixture directory");
        root
    }

    /// A package name the old hand-rolled scan got wrong: a single-quoted name
    /// with a later `[[bin]]` name, and a `name` key with no space around `=`.
    /// 旧的手写扫描会读错的包名：单引号名字后面还跟着 `[[bin]]` 的 name；以及 `name`
    /// 键等号两侧没有空格。
    #[test]
    fn the_package_name_comes_from_cargo_not_from_a_manifest_scan() {
        let root = temporary_root("name");
        let fixtures = [
            (
                "single-quoted",
                "[package]\nname = 'single-quoted'\nversion = \"0.1.0\"\n\n[[bin]]\nname = \"other-bin\"\npath = \"src/main.rs\"\n",
                "single-quoted",
            ),
            (
                "tight",
                "[package]\nname=\"tight\"\nversion = \"0.1.0\"\n\n[lib]\nname = \"different_lib\"\npath = \"src/lib.rs\"\n",
                "tight",
            ),
        ];
        for (directory, manifest, expected) in fixtures {
            let manifest_dir = root.join(directory);
            std::fs::create_dir_all(manifest_dir.join("src")).expect("source directory");
            std::fs::write(manifest_dir.join("Cargo.toml"), manifest).expect("manifest");
            std::fs::write(manifest_dir.join("src/main.rs"), "").expect("binary source");
            std::fs::write(manifest_dir.join("src/lib.rs"), "").expect("library source");
            assert_eq!(
                package_name(&manifest_dir.join("Cargo.toml")).expect("cargo answers"),
                expected,
                "{directory}"
            );
        }
        let _ = std::fs::remove_dir_all(&root);
    }

    /// A virtual manifest names no package, so the identity cannot be guessed:
    /// the caller has to say so instead of falling back to the directory name.
    /// 虚拟 manifest 不命名任何包，身份因此无从猜测：调用方必须说出来，而不是回退到
    /// 目录名。
    #[test]
    fn a_manifest_without_a_package_is_an_error() {
        let root = temporary_root("virtual");
        std::fs::write(
            root.join("Cargo.toml"),
            "[workspace]\nmembers = []\nresolver = \"2\"\n",
        )
        .expect("manifest");
        let error = package_name(&root.join("Cargo.toml"))
            .expect_err("a virtual manifest names no package");
        assert!(error.contains("not a package"), "{error}");
        let _ = std::fs::remove_dir_all(&root);
    }

    /// A path that is not a manifest is an error too, and the message names the
    /// path: this is the case a caller reaches by passing a directory instead of
    /// the manifest file.
    /// 不是清单的路径同样是错误，而且消息点名该路径：调用方传目录而不是清单文件时走到的正是这一种。
    #[test]
    fn a_path_that_is_not_a_manifest_is_an_error() {
        let root = temporary_root("no-manifest");
        let error = package_name(&root).expect_err("a directory is not a manifest");
        assert!(error.contains("cargo metadata failed"), "{error}");
        let _ = std::fs::remove_dir_all(&root);
    }
}
