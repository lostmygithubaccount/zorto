fn main() {
    if let Err(e) = zorto::run(std::env::args()) {
        eprintln!("Error: {e}");
        std::process::exit(1);
    }
}
