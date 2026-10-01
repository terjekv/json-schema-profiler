# Locked dependency license inventory

Reconciled 2026-10-01 against `cargo metadata --locked`. This includes transitive
and development dependencies and target-specific lockfile entries; it is a
metadata inventory, not a bundled collection of license texts. Entries declaring
alternative licenses allow a permissive MIT/Apache option; the inventory does
not select LGPL for an `OR` expression. Unicode, Zlib and other listed notice
requirements still apply to the corresponding dependencies.

The local `schema_analysis` 0.7.0 runtime copy retains its original
[MIT](../licenses/schema_analysis/LICENSE-MIT) and
[Apache-2.0](../licenses/schema_analysis/LICENSE-APACHE) license texts. These copies
also accompany the source patch in the Cargo archive. Its runtime
dependencies are unchanged. The registry copy of the same version is also used
as a development-only differential-test oracle; both copies have the same
declared license shown below. See the [patch provenance](../licenses/schema_analysis/README.md).

| Package | Version | Declared license |
| --- | --- | --- |
| `ahash` | 0.8.12 | MIT OR Apache-2.0 |
| `aho-corasick` | 1.1.5 | Unlicense OR MIT |
| `alloca` | 0.4.0 | MIT |
| `allocator-api2` | 0.2.21 | MIT OR Apache-2.0 |
| `anes` | 0.1.6 | MIT OR Apache-2.0 |
| `anstyle` | 1.0.14 | MIT OR Apache-2.0 |
| `autocfg` | 1.5.1 | Apache-2.0 OR MIT |
| `bincode-next` | 3.1.1 | MIT or Apache-2.0 |
| `bit-set` | 0.8.0 | Apache-2.0 OR MIT |
| `bit-vec` | 0.8.0 | Apache-2.0 OR MIT |
| `bitflags` | 2.13.2 | MIT OR Apache-2.0 |
| `borrow-or-share` | 0.2.4 | MIT-0 |
| `bumpalo` | 3.20.3 | MIT OR Apache-2.0 |
| `bytecount` | 0.6.9 | Apache-2.0/MIT |
| `cast` | 0.3.0 | MIT OR Apache-2.0 |
| `cc` | 1.4.7 | MIT OR Apache-2.0 |
| `cfg-if` | 1.0.5 | MIT OR Apache-2.0 |
| `ciborium` | 0.2.2 | Apache-2.0 |
| `ciborium-io` | 0.2.2 | Apache-2.0 |
| `ciborium-ll` | 0.2.2 | Apache-2.0 |
| `clap` | 4.6.7 | MIT OR Apache-2.0 |
| `clap_builder` | 4.6.7 | MIT OR Apache-2.0 |
| `clap_lex` | 1.1.1 | MIT OR Apache-2.0 |
| `criterion` | 0.8.2 | Apache-2.0 OR MIT |
| `criterion-plot` | 0.8.2 | Apache-2.0 OR MIT |
| `crossbeam-deque` | 0.8.8 | MIT OR Apache-2.0 |
| `crossbeam-epoch` | 0.9.21 | MIT OR Apache-2.0 |
| `crossbeam-utils` | 0.8.23 | MIT OR Apache-2.0 |
| `crunchy` | 0.2.4 | MIT |
| `data-encoding` | 2.11.1 | MIT |
| `derive_more` | 2.1.1 | MIT |
| `derive_more-impl` | 2.1.1 | MIT |
| `displaydoc` | 0.2.7 | MIT OR Apache-2.0 |
| `either` | 1.18.0 | MIT OR Apache-2.0 |
| `either-or-both` | 0.3.1 | Apache-2.0 OR MIT |
| `email_address` | 0.2.9 | MIT |
| `equivalent` | 1.0.2 | Apache-2.0 OR MIT |
| `fancy-regex` | 0.19.2 | MIT |
| `find-msvc-tools` | 0.1.13 | MIT OR Apache-2.0 |
| `fluent-uri` | 0.4.1 | MIT |
| `foldhash` | 0.2.0 | Zlib |
| `fraction` | 0.15.4 | MIT OR Apache-2.0 |
| `futures-core` | 0.3.34 | MIT OR Apache-2.0 |
| `futures-macro` | 0.3.34 | MIT OR Apache-2.0 |
| `futures-task` | 0.3.34 | MIT OR Apache-2.0 |
| `futures-timer` | 3.0.4 | MIT/Apache-2.0 |
| `futures-util` | 0.3.34 | MIT OR Apache-2.0 |
| `getrandom` | 0.3.4 | MIT OR Apache-2.0 |
| `glob` | 0.3.4 | MIT OR Apache-2.0 |
| `gungraun` | 0.19.4 | Apache-2.0 OR MIT |
| `gungraun-macros` | 0.9.1 | Apache-2.0 OR MIT |
| `gungraun-runner` | 0.19.4 | Apache-2.0 OR MIT |
| `half` | 2.7.1 | MIT OR Apache-2.0 |
| `hashbrown` | 0.17.1 | MIT OR Apache-2.0 |
| `heck` | 0.5.0 | MIT OR Apache-2.0 |
| `icu_collections` | 2.3.0 | Unicode-3.0 |
| `icu_locale_core` | 2.3.0 | Unicode-3.0 |
| `icu_normalizer` | 2.3.0 | Unicode-3.0 |
| `icu_normalizer_data` | 2.3.0 | Unicode-3.0 |
| `icu_properties` | 2.3.0 | Unicode-3.0 |
| `icu_properties_data` | 2.3.0 | Unicode-3.0 |
| `icu_provider` | 2.3.1 | Unicode-3.0 |
| `idna` | 1.1.0 | MIT OR Apache-2.0 |
| `idna_adapter` | 1.2.2 | Apache-2.0 OR MIT |
| `indexmap` | 2.14.2 | Apache-2.0 OR MIT |
| `itertools` | 0.13.0 | MIT OR Apache-2.0 |
| `itoa` | 1.0.18 | MIT OR Apache-2.0 |
| `js-sys` | 0.3.105 | MIT OR Apache-2.0 |
| `jsonschema` | 0.49.9 | MIT |
| `jsonschema-macros` | 0.49.9 | MIT |
| `jsonschema-macros-core` | 0.49.9 | MIT |
| `jsonschema-regex` | 0.49.9 | MIT |
| `jsonschema-value` | 0.49.9 | MIT |
| `lazy_static` | 1.5.0 | MIT OR Apache-2.0 |
| `libc` | 0.2.189 | MIT OR Apache-2.0 |
| `litemap` | 0.8.3 | Unicode-3.0 |
| `lock_api` | 0.4.14 | MIT OR Apache-2.0 |
| `memchr` | 2.8.3 | Unlicense OR MIT |
| `micromap` | 0.3.0 | MIT |
| `num` | 0.4.3 | MIT OR Apache-2.0 |
| `num-bigint` | 0.4.8 | MIT OR Apache-2.0 |
| `num-cmp` | 0.1.0 | MIT/Apache-2.0 |
| `num-complex` | 0.4.6 | MIT OR Apache-2.0 |
| `num-integer` | 0.1.47 | MIT OR Apache-2.0 |
| `num-iter` | 0.1.46 | MIT OR Apache-2.0 |
| `num-rational` | 0.4.2 | MIT OR Apache-2.0 |
| `num-traits` | 0.2.19 | MIT OR Apache-2.0 |
| `once_cell` | 1.21.4 | MIT OR Apache-2.0 |
| `oorandom` | 11.1.5 | MIT |
| `ordered-float` | 5.5.0 | MIT |
| `ordermap` | 1.2.2 | Apache-2.0 OR MIT |
| `outref` | 0.5.2 | MIT |
| `page_size` | 0.6.0 | MIT/Apache-2.0 |
| `parking_lot` | 0.12.5 | MIT OR Apache-2.0 |
| `parking_lot_core` | 0.9.12 | MIT OR Apache-2.0 |
| `pastey` | 0.2.3 | MIT OR Apache-2.0 |
| `percent-encoding` | 2.3.2 | MIT OR Apache-2.0 |
| `pin-project-lite` | 0.2.17 | Apache-2.0 OR MIT |
| `plotters` | 0.3.7 | MIT |
| `plotters-backend` | 0.3.7 | MIT |
| `plotters-svg` | 0.3.7 | MIT |
| `potential_utf` | 0.1.6 | Unicode-3.0 |
| `proc-macro-crate` | 3.5.0 | MIT OR Apache-2.0 |
| `proc-macro-error-attr3` | 3.1.1 | MIT OR Apache-2.0 |
| `proc-macro-error3` | 3.1.1 | MIT OR Apache-2.0 |
| `proc-macro2` | 1.0.107 | MIT OR Apache-2.0 |
| `quote` | 1.0.47 | MIT OR Apache-2.0 |
| `r-efi` | 5.3.0 | MIT OR Apache-2.0 OR LGPL-2.1-or-later |
| `rand` | 0.8.8 | MIT OR Apache-2.0 |
| `rand_core` | 0.6.4 | MIT OR Apache-2.0 |
| `rapidhash` | 4.5.1 | MIT OR Apache-2.0 |
| `rayon` | 1.12.0 | MIT OR Apache-2.0 |
| `rayon-core` | 1.13.0 | MIT OR Apache-2.0 |
| `redox_syscall` | 0.5.18 | MIT |
| `ref-cast` | 1.0.27 | MIT OR Apache-2.0 |
| `ref-cast-impl` | 1.0.27 | MIT OR Apache-2.0 |
| `referencing` | 0.49.9 | MIT |
| `regex` | 1.13.1 | MIT OR Apache-2.0 |
| `regex-automata` | 0.4.18 | MIT OR Apache-2.0 |
| `regex-syntax` | 0.8.11 | MIT OR Apache-2.0 |
| `relative-path` | 1.9.3 | MIT OR Apache-2.0 |
| `rstest` | 0.26.1 | MIT OR Apache-2.0 |
| `rstest_macros` | 0.26.1 | MIT OR Apache-2.0 |
| `rustc_version` | 0.4.1 | MIT OR Apache-2.0 |
| `rustversion` | 1.0.23 | MIT OR Apache-2.0 |
| `same-file` | 1.0.6 | Unlicense/MIT |
| `schema_analysis` | 0.7.0 | MIT OR Apache-2.0 |
| `scopeguard` | 1.2.0 | MIT OR Apache-2.0 |
| `semver` | 1.0.28 | MIT OR Apache-2.0 |
| `serde` | 1.0.229 | MIT OR Apache-2.0 |
| `serde_core` | 1.0.229 | MIT OR Apache-2.0 |
| `serde_derive` | 1.0.229 | MIT OR Apache-2.0 |
| `serde_json` | 1.0.151 | MIT OR Apache-2.0 |
| `shlex` | 2.0.1 | MIT OR Apache-2.0 |
| `slab` | 0.4.12 | MIT |
| `smallvec` | 1.16.1 | MIT OR Apache-2.0 |
| `stable_deref_trait` | 1.2.1 | MIT OR Apache-2.0 |
| `strum` | 0.28.0 | MIT |
| `strum_macros` | 0.28.0 | MIT |
| `syn` | 2.0.119 | MIT OR Apache-2.0 |
| `syn` | 3.0.6 | MIT OR Apache-2.0 |
| `synstructure` | 0.14.0 | MIT |
| `tinystr` | 0.8.4 | Unicode-3.0 |
| `tinytemplate` | 1.2.1 | Apache-2.0 OR MIT |
| `toml_datetime` | 1.1.1+spec-1.1.0 | MIT OR Apache-2.0 |
| `toml_edit` | 0.25.15+spec-1.1.0 | MIT OR Apache-2.0 |
| `toml_parser` | 1.1.3+spec-1.1.0 | MIT OR Apache-2.0 |
| `unicode-general-category` | 1.1.0 | Apache-2.0 |
| `unicode-ident` | 1.0.26 | (MIT OR Apache-2.0) AND Unicode-3.0 |
| `unty-next` | 0.1.2 | MIT OR Apache-2.0 |
| `utf8_iter` | 1.0.4 | Apache-2.0 OR MIT |
| `uuid-simd` | 0.8.0 | MIT |
| `version_check` | 0.9.5 | MIT/Apache-2.0 |
| `vsimd` | 0.8.0 | MIT |
| `walkdir` | 2.5.0 | Unlicense/MIT |
| `wasip2` | 1.0.4+wasi-0.2.12 | Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT |
| `wasm-bindgen` | 0.2.128 | MIT OR Apache-2.0 |
| `wasm-bindgen-macro` | 0.2.128 | MIT OR Apache-2.0 |
| `wasm-bindgen-macro-support` | 0.2.128 | MIT OR Apache-2.0 |
| `wasm-bindgen-shared` | 0.2.128 | MIT OR Apache-2.0 |
| `web-sys` | 0.3.105 | MIT OR Apache-2.0 |
| `winapi` | 0.3.9 | MIT/Apache-2.0 |
| `winapi-i686-pc-windows-gnu` | 0.4.0 | MIT/Apache-2.0 |
| `winapi-util` | 0.1.11 | Unlicense OR MIT |
| `winapi-x86_64-pc-windows-gnu` | 0.4.0 | MIT/Apache-2.0 |
| `windows-link` | 0.2.1 | MIT OR Apache-2.0 |
| `windows-sys` | 0.61.2 | MIT OR Apache-2.0 |
| `winnow` | 1.0.4 | MIT |
| `wit-bindgen` | 0.57.1 | Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT |
| `writeable` | 0.6.4 | Unicode-3.0 |
| `yoke` | 0.8.3 | Unicode-3.0 |
| `yoke-derive` | 0.8.4 | Unicode-3.0 |
| `zerocopy` | 0.8.58 | BSD-2-Clause OR Apache-2.0 OR MIT |
| `zerocopy-derive` | 0.8.58 | BSD-2-Clause OR Apache-2.0 OR MIT |
| `zerofrom` | 0.1.8 | Unicode-3.0 |
| `zerofrom-derive` | 0.1.8 | Unicode-3.0 |
| `zerotrie` | 0.2.5 | Unicode-3.0 |
| `zerovec` | 0.11.8 | Unicode-3.0 |
| `zerovec-derive` | 0.11.6 | Unicode-3.0 |
| `zmij` | 1.0.23 | MIT |
