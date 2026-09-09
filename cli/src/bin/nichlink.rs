fn main() {
    if let Err(error) = nichlink_cli::main() {
        eprintln!("nichlink: {error}");
        std::process::exit(1);
    }
}
