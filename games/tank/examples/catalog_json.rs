//! Print Tank Arena's build catalog as JSON, exactly as `catalogJson("tank")` returns
//! it. Regenerates the committed file (a test fails while it is stale):
//!
//! `cargo run -q -p tank --example catalog_json > games/tank/catalog.json`

fn main() {
    let json = serde_json::to_string(&tank::catalog::catalog()).expect("catalog serializes");
    // No trailing newline: the file is the exact `catalogJson` output.
    print!("{json}");
}
