//! Create a host project: the bridge's scaffold write.
//! 创建宿主项目：桥的脚手架写入。
//!
//! The bridge does not render a project itself. It calls
//! `crate::build_time::scaffold::create_project`, the executor `nichlink new` (the
//! CLI) and `submit_new_project` (Studio) both run, so a project an agent scaffolds
//! is the project those two write: the same manifest, build script, source entry,
//! editor snippets, and detected dependency source.
//! 桥不自己渲染项目。它调用 `crate::build_time::scaffold::create_project`——CLI 的
//! `nichlink new` 与 Studio 的 `submit_new_project` 都运行的那个执行器——因此代理脚手架出来的
//! 项目就是那两者写出的项目：同一份清单、构建脚本、源码入口、编辑器 snippet，以及同一个被探测出的
//! 依赖来源。
//!
//! **Preview is the default, and it is the real operation.** A preview runs the same
//! executor in a throwaway directory and prints every path and every byte it produced,
//! so the bytes a preview shows are the bytes `apply: true` writes; only the directory
//! differs, and the reply names the real destination.
//! **预览是默认，而且它就是真实操作。** 预览在一次性目录里运行同一个执行器，并打印它产出的每个路径与
//! 每个字节，因此预览显示的字节就是 `apply: true` 写下的字节；差的只是目录，而回复点名真实的目的地。
//!
//! A destination outside the root this call runs in is refused before anything is
//! created, and the refusal names both paths. On a virtual manifest the root is a
//! workspace of many members, and a relative `directory` is a child of *that* root
//! rather than of whatever directory the process was started in — which is also why the
//! scaffold needs no member's identity: the destination is explicit, and containment in
//! the root is the whole ownership question.
//! 落在这个调用所运行之根以外的目的地会在创建任何东西之前被拒绝，且拒绝会点名两条路径。在虚拟清单上
//! 根是含多个成员的工作区，而相对 `directory` 是**那个**根的子目录，而不是进程恰好在其中启动的目录
//! ——这也正是脚手架不需要任何成员身份的原因：目的地是显式的，而"落在根内"就是归属问题的全部。

use std::path::{Component, Path, PathBuf};

use crate::build_time::scaffold::{self, ProjectKind};
use serde_json::Value;

use crate::mcp::preview::{remove_copy, work_directory};
use crate::mcp::source_index::portable_path;

/// Run one `nichlink.new_project` request, previewing unless `apply` is true.
/// 执行一次 `nichlink.new_project` 请求；除非 `apply` 为真，否则只预览。
pub(crate) fn new_project(root: &Path, arguments: &Value) -> Result<String, String> {
    let directory = text(arguments, "directory")?;
    let package = text(arguments, "package")?;
    let kind = kind(arguments)?;
    // Parsed before anything is written: a bad entry has to be refused while the
    // destination is still untouched.
    // 在任何写入之前解析：坏条目必须在目的地还没被动过的时候就被拒绝。
    let requests = face_requests(arguments)?;
    let apply = arguments
        .get("apply")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let target = destination(root, &directory)?;
    // The executor refuses a non-empty destination itself, and the rule is stated here
    // too: a preview works in a throwaway directory, where the executor's own check
    // would see an empty one and pass, so a preview that let this through would promise
    // a write the apply then refuses.
    // 非空目的地由执行器自己拒绝，而这条规则在这里也要说一遍：预览在一次性目录里工作，执行器自己的
    // 检查在那里看到的是一个空目录、因而会放行；因此放过这一条的预览会承诺一次落盘随后拒绝的写入。
    if target.exists() {
        if !target.is_dir() {
            return Err(format!(
                "REFUSED: {} exists and is not a directory; nichlink.new_project creates one. \
                 Nothing was written",
                target.display()
            ));
        }
        if holds_anything(&target) {
            return Err(format!(
                "REFUSED: {} is not empty, and a new project may only be written into an empty \
                 destination. Nothing was written",
                target.display()
            ));
        }
    }
    // A destination that already exists is a place the caller already has, so the request
    // says `confirm` itself — the same shape `apply delete` uses, for the same reason: the
    // write lands in a directory the caller made, and a failure leaves the partial project
    // in it. A destination that does not exist yet is the new project's own directory, and
    // demanding confirmation for a directory nobody has would only make the tool unusable.
    // 已经存在的目的地是调用方已经持有的地方，因此请求自己说出 `confirm`——与 `apply delete`
    // 同一种形状、同一个理由：这次写入落在调用方建出的目录里，而失败会把半成品项目留在其中。尚不存在
    // 的目的地是新项目自己的目录，而为一个谁都没有的目录要求确认只会让工具没法用。
    if apply && target.exists() && arguments.get("confirm").and_then(Value::as_bool) != Some(true) {
        return Err(format!(
            "new_project requires `confirm: true` for {}: the destination already exists, so this \
             write lands in a directory the caller already has, and a failure leaves the partial \
             project in it. Nothing was written",
            target.display()
        ));
    }
    let source = scaffold::detected_source(
        Path::new(env!("CARGO_MANIFEST_DIR")),
        &std::env::current_exe().unwrap_or_default(),
    );
    if apply {
        if requests.is_empty() {
            scaffold::create_project(&target, &package, kind, &source)?;
            return Ok(applied(root, &target, &package, kind, &produced(&target)?, &[]));
        }
        // Faces make this two steps, so the whole project is built beside the destination and
        // moved in at the end: one rename, and a face the kernel refuses leaves the
        // destination exactly as it was.
        // 面让这次写入变成两步，因此整个项目在目的地旁边建好、最后移进去：一次 rename，而某个面被
        // 内核拒绝时目的地原封不动。
        let staging = staging_directory(&target)?;
        let outcome = scaffold::create_project(&staging, &package, kind, &source)
            .and_then(|()| add_faces(&staging, &requests))
            .and_then(|faces| {
                // A destination that already exists is **empty** (the check above ran), and
                // `rename` cannot replace a directory — so the empty one is removed first and
                // the finished tree takes its name. Without this, the one destination shape the
                // tool always accepted (an empty directory the caller made, `--directory .`)
                // failed with `EBUSY`; the round-13 v2 arm measured exactly that, from the
                // caller's side: destination equal to the root was refused after the whole
                // project had been built.
                // 已经存在的目的地是**空的**（上面的检查跑过），而 `rename` 无法替换一个目录——因此先把
                // 那个空目录移走，让成品接管它的名字。没有这一步，这个工具一直接受的唯一一种目的地形状
                // （调用方建好的空目录、`--directory .`）会以 `EBUSY` 失败；第十三轮 v2 的臂正是从调用方
                // 那一侧量到了它：目的地等于根时，整个项目已经建好却被拒绝。
                if target.exists() {
                    std::fs::remove_dir(&target).map_err(|error| {
                        format!("cannot clear the empty {}: {error}", target.display())
                    })?;
                }
                std::fs::rename(&staging, &target)
                    .map_err(|error| {
                        format!(
                            "cannot move {} to {}: {error}",
                            staging.display(),
                            target.display()
                        )
                    })
                    .map(|()| faces)
            });
        return match outcome {
            Ok(faces) => Ok(applied(
                root,
                &target,
                &package,
                kind,
                &produced(&target)?,
                &faces,
            )),
            Err(error) => {
                let _ = std::fs::remove_dir_all(&staging);
                Err(format!(
                    "{error}; nothing was written to {}",
                    target.display()
                ))
            }
        };
    }
    // The preview runs the executor for real, in a throwaway directory whose name is the
    // destination's, so the paths it reports are the paths the apply writes, one directory
    // up.
    // 预览在一次性目录里真实运行执行器，目录名与目的地相同，因此它报告的路径就是落盘会写的路径，
    // 只是上移了一层目录。
    let work = work_directory("new-project")?;
    let staged = work.join(target.file_name().map_or_else(
        || std::ffi::OsString::from("project"),
        std::ffi::OsStr::to_os_string,
    ));
    let outcome = scaffold::create_project(&staged, &package, kind, &source)
        .and_then(|()| add_faces(&staged, &requests))
        .and_then(|faces| produced(&staged).map(|files| (files, faces)));
    let report = outcome.map(|(files, faces)| preview(root, &target, &package, kind, &files, &faces));
    remove_copy(root, &work);
    report
}

/// What an apply says: where the project landed, every file it wrote, the faces it now
/// derives, and the call that says whether it compiles.
/// 落盘说的话：项目落在哪、写下的每个文件、它现在推导出的面，以及那句"它能不能编译"的调用。
fn applied(
    root: &Path,
    target: &Path,
    package: &str,
    kind: ProjectKind,
    files: &[(String, String)],
    faces: &[String],
) -> String {
    let mut report = format!(
        "applied: created the {kind} project `{package}` at {}\n",
        target.display()
    );
    report.push_str(&format!(
        "root {}: the destination is inside it\n",
        root.display()
    ));
    report.push_str(&format!(
        "wrote {} file(s) under {}:\n",
        files.len(),
        target.display()
    ));
    for (relative, _) in files {
        report.push_str(&format!("  {relative}\n"));
    }
    report.push_str(&faces_block(faces));
    report.push_str(
        "the scaffolded manifest declares its own `[workspace]`, so this project is not a \
         member of the root above\n",
    );
    report
}

/// The faces a project derives, as the block every reply about creation carries.
/// 项目推导出的面，写作每条创建回复都带的那一块。
///
/// The faces are reported **here** rather than left to a second `registry` call: the round-13
/// benchmark measured the create-a-project workflow as five to six round trips, and the tree is
/// the fact the caller needs next — one call that creates and reports is the consolidation the
/// community guidance asks for.
/// 面**在这里**报出来，而不是留给第二次 `registry` 调用：第十三轮量到"新建项目"这条工作流要五到
/// 六个往返，而这棵树正是调用方接下来需要的事实——一次调用既创建又报告，就是社区指引要求的那种合并。
fn faces_block(faces: &[String]) -> String {
    if faces.is_empty() {
        return String::new();
    }
    let mut block = format!("faces {} (this project derives them now):\n", faces.len());
    for face in faces {
        block.push_str(face);
        block.push('\n');
    }
    block.push_str(
        "next   `check {face: \"default\"}` is the run that says whether this compiles\n",
    );
    block
}

/// The registration faces the request asks for **at creation time**.
/// 请求在**创建时**就要的那些注册面。
///
/// One entry is `{"fields": {...}, "parent": "<node>"}` — the same shape `apply add` takes
/// without its `action`, because the entries run through that very path. `parent` defaults to
/// the project root, which is what "create a project with these two objects" means. The whole
/// array is validated here, before anything is written, so a typo in the third entry cannot
/// leave a project behind with two faces and a half-written third.
/// 一条是 `{"fields": {...}, "parent": "<node>"}`——与 `apply add` 相同、只少了它的 `action`，
/// 因为这些条目正是走那条路径。`parent` 默认是项目根，而"用这两个对象建一个项目"就是它。整批在这里、
/// 在任何写入之前校验，因此第三条的拼写错误不会留下一个"有两个面、第三个写了一半"的项目。
fn face_requests(arguments: &Value) -> Result<Vec<Value>, String> {
    let Some(value) = arguments.get("faces") else {
        return Ok(Vec::new());
    };
    let Some(items) = value.as_array() else {
        return Err(
            "`faces` must be an array of `{\"fields\": {...}, \"parent\": \"<node>\"}` objects, one per \
             registration face to create — e.g. \"faces\":[{\"fields\":{\"module\":\"button\",\
             \"kind\":\"Button\"}},{\"fields\":{\"module\":\"slider\",\"kind\":\"Slider\"}}]"
                .to_owned(),
        );
    };
    if items.is_empty() {
        return Err(
            "`faces` is empty: drop the key to scaffold a project with no registration face yet"
                .to_owned(),
        );
    }
    let mut requests = Vec::new();
    for (index, item) in items.iter().enumerate() {
        let Some(object) = item.as_object() else {
            return Err(format!(
                "`faces[{index}]` must be an object with `fields`; one entry is \
                 {{\"fields\": {{\"module\": \"button\", \"kind\": \"Button\"}}}}"
            ));
        };
        if !object.get("fields").is_some_and(Value::is_object) {
            return Err(format!(
                "`faces[{index}]` requires `fields`, one JSON object holding the new face's fields \
                 — accepted shape: {{\"module\": \"<snake_case>\", \"kind\": \"<Kind>\"}}"
            ));
        }
        let mut request = serde_json::Map::new();
        for (key, value) in object {
            match key.as_str() {
                "fields" | "parent" => {
                    request.insert(key.clone(), value.clone());
                }
                other => {
                    return Err(format!(
                        "`faces[{index}].{other}` is not a key this shape takes; one entry is \
                         {{\"fields\": {{...}}, \"parent\": \"<node>\"}} — the same shape \
                         `apply add` takes, without `action`"
                    ));
                }
            }
        }
        requests.push(Value::Object(request));
    }
    Ok(requests)
}

/// Add every face the request asked for, **through the executor `apply add` runs**.
/// 把请求要的每个面加进去——**走 `apply add` 用的那个执行器**。
///
/// In order, so a folder face added earlier can be a later entry's `parent`; the fields are the
/// ones that path already validates, which is why this is a loop over requests rather than a
/// second mapping of field names.
/// 按顺序，因此先加进去的文件夹面可以是后一条的 `parent`；字段校验用的是那条路径已经有的那一套，
/// 因此这里是对请求的循环，而不是第二份字段名映射。
fn add_faces(staging: &Path, requests: &[Value]) -> Result<Vec<String>, String> {
    let namespace = crate::mcp::registry::namespace(staging)?;
    for (index, request) in requests.iter().enumerate() {
        crate::mcp::apply::run_add(staging, &namespace, request)
            .map_err(|error| format!("`faces[{index}]` was refused: {error}"))?;
    }
    let faces = crate::build_time::face_views(staging, &namespace)?;
    Ok(faces
        .iter()
        .map(|face| format!("  {}  {}  {}", face.path, face.kind, face.source))
        .collect())
}

/// A sibling of the destination to build the project in, so the finished tree can be moved
/// into place with **one** rename on the same filesystem.
/// 目的地的一个同级目录，用来在里面把项目建好，于是成品可以用**一次** rename 移到位（同一文件系统）。
///
/// A face the kernel refuses makes this a partial project, and a partial scaffold is worse than
/// none — the reason `create_project` already removes what it wrote. Building beside the
/// destination extends that promise to the faces: nothing exists under the destination until the
/// whole thing is there.
/// 某个面被内核拒绝会让它变成半成品，而半成品项目比没有更糟——这正是 `create_project` 失败时移除自己
/// 写下的东西的理由。在目的地旁边建，把这个承诺扩展到面：在整棵树都在之前，目的地下面什么都没有。
fn staging_directory(target: &Path) -> Result<PathBuf, String> {
    let parent = target
        .parent()
        .ok_or_else(|| format!("{} has no parent directory", target.display()))?;
    let name = target
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| "project".to_owned());
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|elapsed| elapsed.as_nanos())
        .unwrap_or(0);
    Ok(parent.join(format!(".nichlink-new-{name}-{stamp}")))
}

/// The one required string argument named `key`.
/// 名为 `key` 的那一个必填字符串参数。
fn text(arguments: &Value, key: &str) -> Result<String, String> {
    match arguments.get(key).and_then(Value::as_str) {
        Some(value) if !value.trim().is_empty() => Ok(value.trim().to_owned()),
        Some(_) => Err(format!("`{key}` must not be empty")),
        None => Err(format!("nichlink.new_project requires `{key}`")),
    }
}

/// The `kind` argument, which the executor has two of and no default for.
/// `kind` 参数；执行器只有两种，且没有默认值。
fn kind(arguments: &Value) -> Result<ProjectKind, String> {
    match arguments.get("kind").and_then(Value::as_str) {
        Some("binary") => Ok(ProjectKind::Binary),
        Some("library") => Ok(ProjectKind::Library),
        Some(other) => Err(format!(
            "`kind` must be `binary` or `library`, not `{other}`"
        )),
        None => Err("nichlink.new_project requires `kind` (`binary` or `library`)".to_owned()),
    }
}

/// The destination, refused unless it is inside the root this call runs in.
/// 目的地；除非它落在本次调用所运行之根内部，否则拒绝。
fn destination(root: &Path, requested: &str) -> Result<PathBuf, String> {
    let path = Path::new(requested);
    // `..` is refused by name rather than resolved: a destination that walks out and back
    // in again is a destination whose spelling does not say where it lands, and the reply
    // below promises a path a reader can check.
    // `..` 按名拒绝而不是被解析：一个走出去又绕回来的目的地，其拼法并没有说出它落在哪里，而下面的
    // 回复承诺的是一条读者能核对的路径。
    if path
        .components()
        .any(|component| matches!(component, Component::ParentDir))
    {
        return Err(format!(
            "REFUSED: `{requested}` steps out of the root with `..`; a new project is written \
             inside the root this call runs in ({}). Nothing was written",
            root.display()
        ));
    }
    let target = if path.is_absolute() {
        path.to_path_buf()
    } else {
        root.join(path)
    };
    if !inside(root, &target) {
        return Err(format!(
            "REFUSED: `{requested}` resolves to {}, which is outside the root this call runs in \
             ({}); nichlink.new_project writes only inside that root. Nothing was written",
            target.display(),
            root.display()
        ));
    }
    Ok(target)
}

/// Whether `candidate` is inside `root`, even when it does not exist yet.
/// `candidate` 是否落在 `root` 之内——即使它尚不存在。
///
/// The longest existing prefix decides, and it is canonicalized before the comparison, so
/// a symbolic link in the middle of the path cannot carry the destination out of the root
/// (the failure mode a purely lexical check has). Only the names past that prefix are
/// added back, and they were checked for `..` before this ran.
/// 最长已存在前缀说了算，而它在比较之前被规范化，因此路径中段的符号链接无法把目的地带出根外（纯词法
/// 检查的失效方式）。只有那个前缀之后的名称被加回去，而它们在本次调用之前已被检查过 `..`。
fn inside(root: &Path, candidate: &Path) -> bool {
    let root = std::fs::canonicalize(root).unwrap_or_else(|_| root.to_path_buf());
    let mut existing = candidate.to_path_buf();
    let mut tail = Vec::new();
    while !existing.exists() {
        let Some(name) = existing.file_name() else {
            return false;
        };
        tail.push(name.to_os_string());
        let Some(parent) = existing.parent() else {
            return false;
        };
        existing = parent.to_path_buf();
    }
    let mut resolved = std::fs::canonicalize(&existing).unwrap_or(existing);
    for name in tail.iter().rev() {
        resolved.push(name);
    }
    resolved.starts_with(&root)
}

/// Whether `path` holds any entry at all.
/// `path` 里是否有任何条目。
fn holds_anything(path: &Path) -> bool {
    std::fs::read_dir(path)
        .map(|mut entries| entries.next().is_some())
        .unwrap_or(true)
}

/// Every file the executor produced under `root`, as `(portable relative path, text)`.
/// 执行器在 `root` 下产出的每个文件，写作 `(可移植相对路径, 文本)`。
fn produced(root: &Path) -> Result<Vec<(String, String)>, String> {
    let mut files = Vec::new();
    visit_files(root, root, &mut files)?;
    files.sort();
    Ok(files)
}

/// The recursive half of [`produced`].
/// [`produced`] 的递归那一半。
fn visit_files(
    root: &Path,
    directory: &Path,
    files: &mut Vec<(String, String)>,
) -> Result<(), String> {
    let entries = std::fs::read_dir(directory)
        .map_err(|error| format!("cannot read {}: {error}", directory.display()))?;
    for entry in entries {
        let entry = entry.map_err(|error| format!("cannot read a directory entry: {error}"))?;
        let path = entry.path();
        if path.is_dir() {
            visit_files(root, &path, files)?;
        } else if path.is_file() {
            let relative = path
                .strip_prefix(root)
                .map_err(|error| format!("cannot name {}: {error}", path.display()))?;
            let text = std::fs::read_to_string(&path)
                .map_err(|error| format!("cannot read {}: {error}", path.display()))?;
            files.push((portable_path(relative), text));
        }
    }
    Ok(())
}

/// What a preview says: the destination, the root it is inside, and every byte the
/// executor produced in the throwaway directory it actually ran in.
/// 预览说的话：目的地、它所在的根，以及执行器在它真正运行的那个一次性目录里产出的每一个字节。
fn preview(
    root: &Path,
    target: &Path,
    package: &str,
    kind: ProjectKind,
    files: &[(String, String)],
    faces: &[String],
) -> String {
    let mut report = format!(
        "preview: nichlink.new_project would create the {kind} project `{package}` at {}\n",
        target.display()
    );
    report.push_str(&format!(
        "root {}: the destination is inside it\n",
        root.display()
    ));
    report.push_str(&format!(
        "stop   this project exists when `cargo check` compiles it and `registry` lists its \
         faces; the preview below shows every byte that would be written, and nothing is written \
         until `apply: true`\n\
         nothing was written; `apply: true` writes {} file(s) under {}:\n",
        files.len(),
        target.display()
    ));
    if !faces.is_empty() {
        report.push_str(&faces_block(faces));
    }
    for (relative, content) in files {
        report.push_str(&format!("+ {relative}\n"));
        for line in content.lines() {
            report.push_str(&format!("+{line}\n"));
        }
    }
    report.push_str(
        "these bytes come from the same executor, run in a throwaway directory; the editor \
         snippets under .vscode/ are part of the project the executor writes\n",
    );
    report
}

#[cfg(test)]
#[path = "new_project_tests.rs"]
mod new_project_tests;
