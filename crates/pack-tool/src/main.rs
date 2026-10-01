//! The `msime-pack` binary; `msime_pack_tool` documents the command line.

fn main() {
    // OS strings rather than `String`: a pack path need not be UTF-8.
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    let status = msime_pack_tool::run(&args, &mut std::io::stdout(), &mut std::io::stderr());
    std::process::exit(status);
}
