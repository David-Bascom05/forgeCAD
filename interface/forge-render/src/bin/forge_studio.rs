//! The Render Studio from the command line: `forge-studio --help`.

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if let Err(e) = forge_render::studio::shot::main_args(&args) {
        eprintln!("{e}");
        std::process::exit(2);
    }
}
