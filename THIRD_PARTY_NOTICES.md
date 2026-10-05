# Third-party notices

This file lists third-party software compiled into what this repository ships:

- the browser build `web/pkg/engine_wasm_bg.wasm` and its glue `web/pkg/engine_wasm.js`, served on GitHub Pages;
- the `engine-cli` binary, if distributed.

Build-time-only crates (proc macros such as `serde_derive`, `syn`, `quote`, `proc-macro2`, `unicode-ident`, `wasm-bindgen-macro`, `clap_derive`, `heck`, `rustversion`) are not part of the shipped output and are not listed.

Each crate below is used under the license shown. Where a crate offers a choice, we use it under the MIT License or the Apache License 2.0. The full texts of both are in [LICENSE-MIT](LICENSE-MIT) and [LICENSE-APACHE](LICENSE-APACHE); the MIT text applies with each crate's own copyright line below. None of these crates includes an Apache-2.0 `NOTICE` file.

Regenerate with `cargo tree -e normal -p engine-wasm --target wasm32-unknown-unknown` and `cargo tree -e normal -p engine-cli` when dependencies change. `cargo deny check licenses` must pass (see `deny.toml`).

## In the browser build (engine-wasm)

| Crate | Version | License (SPDX) | Copyright | Source |
|---|---|---|---|---|
| bumpalo | 3.20.3 | MIT OR Apache-2.0 | Copyright (c) 2019 Nick Fitzgerald | https://github.com/fitzgen/bumpalo |
| cfg-if | 1.0.5 | MIT OR Apache-2.0 | Copyright (c) 2014 Alex Crichton | https://github.com/rust-lang/cfg-if |
| glam | 0.29.3 | MIT OR Apache-2.0 | Copyright 2020 Cameron Hart | https://github.com/bitshifter/glam-rs |
| itoa | 1.0.18 | MIT OR Apache-2.0 | (no copyright line in license file; see repository) | https://github.com/dtolnay/itoa |
| log | 0.4.34 | MIT OR Apache-2.0 | Copyright (c) 2014 The Rust Project Developers | https://github.com/rust-lang/log |
| memchr | 2.8.3 | Unlicense OR MIT | Copyright (c) 2015 Andrew Gallant | https://github.com/BurntSushi/memchr |
| once_cell | 1.21.4 | MIT OR Apache-2.0 | (no copyright line in license file; see repository) | https://github.com/matklad/once_cell |
| ppv-lite86 | 0.2.21 | MIT OR Apache-2.0 | Copyright (c) 2019 The CryptoCorrosion Contributors; Copyright 2019 The CryptoCorrosion Contributors | https://github.com/cryptocorrosion/cryptocorrosion |
| rand_chacha | 0.9.0 | MIT OR Apache-2.0 | Copyright (c) 2014 The Rust Project Developers; Copyright 2018 Developers of the Rand project | https://github.com/rust-random/rand |
| rand_core | 0.9.5 | MIT OR Apache-2.0 | Copyright (c) 2014 The Rust Project Developers; Copyright 2018 Developers of the Rand project | https://github.com/rust-random/rand |
| serde | 1.0.229 | MIT OR Apache-2.0 | (no copyright line in license file; see repository) | https://github.com/serde-rs/serde |
| serde_core | 1.0.229 | MIT OR Apache-2.0 | (no copyright line in license file; see repository) | https://github.com/serde-rs/serde |
| serde_json | 1.0.151 | MIT OR Apache-2.0 | (no copyright line in license file; see repository) | https://github.com/serde-rs/json |
| wasm-bindgen | 0.2.100 | MIT OR Apache-2.0 | Copyright (c) 2014 Alex Crichton | https://github.com/rustwasm/wasm-bindgen |
| wasm-bindgen-shared | 0.2.100 | MIT OR Apache-2.0 | Copyright (c) 2014 Alex Crichton | https://github.com/rustwasm/wasm-bindgen/tree/master/crates/shared |
| zerocopy | 0.8.59 | BSD-2-Clause OR Apache-2.0 OR MIT | Copyright 2019 The Fuchsia Authors.; Copyright 2023 The Fuchsia Authors | https://github.com/google/zerocopy |
| zmij | 1.0.23 | MIT | (no copyright line in license file; see repository) | https://github.com/dtolnay/zmij |

## Additionally in engine-cli

| Crate | Version | License (SPDX) | Copyright | Source |
|---|---|---|---|---|
| anstream | 1.0.0 | MIT OR Apache-2.0 | Copyright (c) Individual contributors | https://github.com/rust-cli/anstyle.git |
| anstyle | 1.0.14 | MIT OR Apache-2.0 | Copyright (c) Individual contributors | https://github.com/rust-cli/anstyle.git |
| anstyle-parse | 1.0.0 | MIT OR Apache-2.0 | Copyright (c) Individual contributors | https://github.com/rust-cli/anstyle.git |
| anstyle-query | 1.1.5 | MIT OR Apache-2.0 | Copyright (c) Individual contributors | https://github.com/rust-cli/anstyle.git |
| clap | 4.6.7 | MIT OR Apache-2.0 | Copyright (c) Individual contributors | https://github.com/clap-rs/clap |
| clap_builder | 4.6.7 | MIT OR Apache-2.0 | Copyright (c) Individual contributors | https://github.com/clap-rs/clap |
| clap_lex | 1.1.1 | MIT OR Apache-2.0 | Copyright (c) Individual contributors | https://github.com/clap-rs/clap |
| colorchoice | 1.0.5 | MIT OR Apache-2.0 | Copyright (c) Individual contributors | https://github.com/rust-cli/anstyle.git |
| is_terminal_polyfill | 1.70.2 | MIT OR Apache-2.0 | Copyright (c) Individual contributors | https://github.com/polyfill-rs/is_terminal_polyfill |
| strsim | 0.11.1 | MIT | Copyright (c) 2015 Danny Guo; Copyright (c) 2016 Titus Wormer <tituswormer@gmail.com>; Copyright (c) 2018 Akash Kurdekar | https://github.com/rapidfuzz/strsim-rs |
| utf8parse | 0.2.2 | Apache-2.0 OR MIT | Copyright (c) 2016 Joe Wilm | https://github.com/alacritty/vte |

## Also included

- `web/pkg/engine_wasm.js` is JavaScript glue generated by `wasm-bindgen-cli` 0.2.100 (MIT OR Apache-2.0, https://github.com/rustwasm/wasm-bindgen).
