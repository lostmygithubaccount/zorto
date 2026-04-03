# Rust API

Zorto's Rust API is documented on [docs.rs](https://docs.rs/zorto-core).

## Crates

| Crate | docs.rs | crates.io | Description |
|-------|---------|-----------|-------------|
| `zorto-core` | [docs](https://docs.rs/zorto-core) | [crate](https://crates.io/crates/zorto-core) | Core library: site model, build pipeline, rendering |
| `zorto` | [docs](https://docs.rs/zorto) | [crate](https://crates.io/crates/zorto) | CLI binary + preview server |

## Quick start

```toml
# Cargo.toml
[dependencies]
zorto-core = "0.13"
```

```rust
use std::path::Path;
use zorto_core::site::Site;

fn main() -> anyhow::Result<()> {
    let root = Path::new("my-site");
    let output = root.join("public");
    let mut site = Site::load(root, &output, false /* drafts */)?;
    site.build()?;
    Ok(())
}
```

## Key types

| Module | Type | Description |
|--------|------|-------------|
| `zorto_core::site` | `Site` | Loaded site with config, sections, pages |
| `zorto_core::config` | `Config` | Parsed `config.toml` |
| `zorto_core::content` | `Page` | Content page with frontmatter and rendered HTML |
| `zorto_core::content` | `Section` | Section with child pages |
| `zorto_core::themes` | `Theme` | Built-in theme enum |

For full API documentation, type signatures, and examples, see [docs.rs/zorto-core](https://docs.rs/zorto-core).
