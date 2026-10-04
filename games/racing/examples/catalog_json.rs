//! Print Racing's build catalog as JSON, exactly as `catalogJson("racing")` returns
//! it. Regenerates the committed file (a test fails while it is stale):
//!
//! `cargo run -q -p racing --example catalog_json > games/racing/catalog.json`

fn main() {
    let json = serde_json::to_string(&racing::catalog::catalog()).expect("catalog serializes");
    // No trailing newline: the file is the exact `catalogJson` output.
    print!("{json}");
}
