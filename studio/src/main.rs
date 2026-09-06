//! NichLink Studio executable entry.
//! NichLink Studio 可执行入口。

#[path = "studio/studio.rs"]
mod studio;

fn main() -> std::io::Result<()> {
    studio::launch()
}
