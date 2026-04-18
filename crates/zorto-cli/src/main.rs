fn main() {
    if let Err(e) = zorto::run(std::env::args()) {
        if let Some(exit) = e.downcast_ref::<zorto::CliExit>() {
            std::process::exit(exit.code());
        }
        // `{:#}` flattens anyhow's error chain, so `.context(...)` frames
        // surface the underlying cause (e.g. "in foo.md: TOML parse error
        // at line 2, column 9...") instead of only the outermost context.
        eprintln!("Error: {e:#}");
        std::process::exit(1);
    }
}
