fn main() {
    if let Err(e) = zorto::run(std::env::args()) {
        if let Some(exit) = e.downcast_ref::<zorto::CliExit>() {
            std::process::exit(exit.code());
        }
        eprintln!("Error: {e}");
        std::process::exit(1);
    }
}
