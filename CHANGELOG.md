# Changelog

## 0.3.0 (2026-11-01)

- Replaces the fake JSON-with-`.1o` experiment codec with real `core-1o` parsing and schema validation.
- Ships `schema/1o.version.json` and `schema/experiment/v1.schema.json`, and embeds the same experiment schema for standalone validation.
- Adds equivalent 1o, JSON, TOML, and YAML/YML experiment-spec codecs and cross-format equivalence tests.
- Adds bounded and unbounded mutation modes. Bounded mode refuses raw intervention commands.
- Adds Code Passenger as the standalone typed mutation backend: declared `ChangeSpec` transforms run through Code Passenger's Simple Impl/XCCUTE line in-process and their transform records become experiment evidence.
- Keeps PathGen responsible for experiment storage rather than source mutation.
- Writes run results as real JSON instead of JSON hidden behind a `.1o` suffix.
- Raises the MSRV to Rust 1.90 for Code Passenger 0.4.9.
- Leaves the Custodian backend for the dedicated integration pass.

## 0.2.0 (2026-11-01)

- Declares the `experiments` CLI through `simple_impl_derive` and `xccute_contract`.
- Lowers dynamic protocol, verification, and measurement operations through
  `xccute_runtime::CommandSpec` instead of constructing raw commands directly.
- Replaces the spawned `differences` process with the typed Differences
  simple_impl/xccute library surface and structured result capture.
- Adds explicit baseline/candidate measurement phases so observations are
  collected at the correct experimental point.
- Bumps experiment/result schemas to v2.
- Keeps scientific evaluation and promotion separate.
- Leaves Custodian-specific execution for the dedicated Custodian integration pass.

## 0.1.1 (2026-11-01)

- Adds GitHub Actions trusted publishing so releases are published to crates.io by CI after tests and clippy pass.

## 0.1.0 (2026-11-01)

- Adds declarative experiment specs and `.1o` JSON codec.
- Adds protocol, verification, measurement plans, observations, evidence, and durable results.
- Integrates `scientific-method` for hypothesis evaluation.
- Uses PathGen for experiment data/state roots and centralizes all experiment subpaths.
- Adds external Differences evidence capture behind the `differences` feature.
- Adds the embedded Custodian seam with `custodian_kit` and `differences_kit` side by side.
- Keeps scientific outcome distinct from candidate promotion.

## 0.0.1 (2026-11-01)

- Initial crate-name publication.
