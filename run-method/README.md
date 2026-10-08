# nichlink-run-method

A **compatibility shell**: the host-facing crate name this surface had before the nine-to-three
merge. It re-exports `nichlink-toolchain`, so a host written against the old name keeps compiling
unchanged, and there is still one implementation behind it.

新代码请直接依赖 `nichlink-toolchain`；本壳只为已经在世界上的清单而存在。
