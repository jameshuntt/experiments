# Changelog

## 0.4.0 (2026-11-03)

Campaigns: experiments composed over an evolving accepted state, and experimental designs over factors, as one
document, one plan and one result. A campaign is planned here and governed by a host (Custodian's `experiment
campaign`); nothing in this crate runs a campaign by itself, and the hosted single experiment (`ExperimentRunner::run_with`)
is unchanged. Minor, not patch: `CommandSpec` and `CommandRecord` have a new public field (`env`), so a struct literal
written against 0.3 does not build; nothing is removed. Links are pinned to 017f082.

- **The document** ([spec.rs L25](https://github.com/jameshuntt/experiments/blob/017f082/src/campaign/spec.rs#L25)): `CampaignSpec`, schema `experiments.campaign.v1`, canonical file
  `v1.<name>.1o.campaign`. Real 1o through `core-1o`: the shipped `schema/campaign/v1.schema.json` declares each step
  once (`x-1o-step-schemas`) and every step node, at any depth, is validated against it before it is read
  ([codec.rs L244](https://github.com/jameshuntt/experiments/blob/017f082/src/codec.rs#L244)); the steps keep their written order across kinds ([codec.rs L320](https://github.com/jameshuntt/experiments/blob/017f082/src/codec.rs#L320), [codec.rs L461](https://github.com/jameshuntt/experiments/blob/017f082/src/codec.rs#L461)).
  JSON, TOML, YAML and YML read the same type; every type is `deny_unknown_fields`, so a typo is refused in every
  format and never becomes a factor or a step. A step is a one-key map in every format ([spec.rs L284](https://github.com/jameshuntt/experiments/blob/017f082/src/campaign/spec.rs#L284)).
- **One plan** ([plan.rs L131](https://github.com/jameshuntt/experiments/blob/017f082/src/campaign/plan.rs#L131), [plan.rs L93](https://github.com/jameshuntt/experiments/blob/017f082/src/campaign/plan.rs#L93)): `run`, `sequence`, `parallel`, `repeat`,
  `sweep`, and the flat form's `after` (execution dependencies) and `from` (one state parent), normalized into nodes
  with a stable id, the experiment's digest (the spec as read and every change spec's bytes), dependencies, state
  parent, workspace (`shared`, `isolated`, `continue`), full assignment, controls, blocks, replicate, group, design,
  rigor, policy and binding; a digest over all of it. Ordered dependencies first, ties by declaration
  ([plan.rs L927](https://github.com/jameshuntt/experiments/blob/017f082/src/campaign/plan.rs#L927)). A sequence and its flat form are one plan with one digest.
- **Refused before anything runs** ([plan.rs L480](https://github.com/jameshuntt/experiments/blob/017f082/src/campaign/plan.rs#L480), [plan.rs L1038-L1058](https://github.com/jameshuntt/experiments/blob/017f082/src/campaign/plan.rs#L1038-L1058)): duplicate ids,
  missing or malformed experiments, missing references, cycles, `from` a parallel/repeat/sweep, two nodes evolving one
  workspace from one state, a parallel on the shared workspace, zero repeats, an undeclared factor or block, a level
  not declared, a factor no experiment declares as an independent variable (unless scope `campaign`), a factor set
  twice, mixed-kind or duplicate levels, name collisions, impossible designs, more than 100000 trials, unbounded
  mutation in the shared workspace, an experiment whose target is not the governed root, a command that names a
  binding's variable itself; strict also refuses an unbounded experiment and a declared control with no value.
- **Designs** ([plan.rs L814](https://github.com/jameshuntt/experiments/blob/017f082/src/campaign/plan.rs#L814)): `factorial` (every combination, first factor slowest),
  `one_factor_at_a_time` (the reference, then each factor moved alone from its `reference`), `explicit` (named
  treatments); each treatment repeated and crossed with the named blocks. Blocks are recorded apart from the
  assignment and are never a treatment.
- **The binding** ([bind.rs L32](https://github.com/jameshuntt/experiments/blob/017f082/src/campaign/bind.rs#L32), [bind.rs L52](https://github.com/jameshuntt/experiments/blob/017f082/src/campaign/bind.rs#L52)): a trial's assignment reaches every command
  of its experiment as environment variables through `CommandSpec::env` ([model.rs L29](https://github.com/jameshuntt/experiments/blob/017f082/src/model.rs#L29)):
  `EXPERIMENT_CAMPAIGN`, `EXPERIMENT_TRIAL`, `EXPERIMENT_REPLICATE`, `EXPERIMENT_TREATMENT`,
  `EXPERIMENT_FACTOR_<NAME>`, `EXPERIMENT_CONTROL_<NAME>`, `EXPERIMENT_BLOCK_<NAME>`, each level in one text form; no
  word is templated and no shell sees it. The standalone host gives them to the program ([runner.rs L358](https://github.com/jameshuntt/experiments/blob/017f082/src/runner.rs#L358)) and
  `CommandRecord::env` keeps them ([model.rs L193](https://github.com/jameshuntt/experiments/blob/017f082/src/model.rs#L193)). The experiment schema takes `env { name value }` on
  every command.
- **The runner** ([run.rs L381](https://github.com/jameshuntt/experiments/blob/017f082/src/campaign/run.rs#L381), [run.rs L329](https://github.com/jameshuntt/experiments/blob/017f082/src/campaign/run.rs#L329), [run.rs L441](https://github.com/jameshuntt/experiments/blob/017f082/src/campaign/run.rs#L441)): promotion
  decides the state (promote accepted, do-not-promote put back, undecided put back unless `on_undecided accept`), an
  undeclared change is a finding (exploratory), a finding and not accepted (controlled), a refusal (strict); a node
  waiting for one that did not complete is skipped with why; `on_findings`, `on_refusal`, `on_error`.
  `CampaignRunner::resume` ([run.rs L233](https://github.com/jameshuntt/experiments/blob/017f082/src/campaign/run.rs#L233)) is refused when the campaign, an experiment, a factor, a
  design or the plan moved; completed nodes keep their results, the rest run again.
- **The result** ([result.rs L213](https://github.com/jameshuntt/experiments/blob/017f082/src/campaign/result.rs#L213), [result.rs L314](https://github.com/jameshuntt/experiments/blob/017f082/src/campaign/result.rs#L314)): `experiments.campaign-result.v1`, every
  node as planned and as it ran (start, candidate and accepted digests, result path and digest, receipts, restores),
  the order things ran in, the workspaces' accepted states, tools, what the host enforced, and the summaries of each
  repeat and sweep by treatment and block and by each level: counts, n, mean, median, min, max and sample standard
  deviation ([result.rs L300](https://github.com/jameshuntt/experiments/blob/017f082/src/campaign/result.rs#L300)). No significance is claimed.
- `pub use scientific_method` and `VERSION`; `examples/v1.matrix.1o.campaign` (2 × 4 × 3 factorial, ten repeats, 240
  trials).
- **CI** ([ci.yml L69](https://github.com/jameshuntt/experiments/blob/017f082/.github/workflows/ci.yml#L69)): `consumers` tests custodian (its branch of this push's name, else its
  default) against this checkout, with its siblings and the writer.

Tests: `tests/campaign.rs` (20: formats and refusals at every depth, sequence ≡ flat, fifty ordered nodes, parallel,
repeat, the 240-trial factorial, explicit ≡ factorial assignments, one-factor-at-a-time, blocks, 39 refusals, strict
and controlled, the binding through a real run, decisions, stops, resume, summaries); `tests/formats.rs` round-trips
`env` through 1o, JSON, TOML, YAML and YML. `cargo test --all-features`: 53 passed, 0 failed (stable and Rust 1.90);
no-default, differences-only and code-passenger-only shapes pass; `clippy --all-targets --all-features -D warnings`
and rustdoc `-D warnings` clean; `cargo package` builds and its tests pass.

## 0.3.2 (2026-11-01)

- Adds the host seam a governing host runs an experiment through: `ExperimentHost` carries out each protocol,
  measurement and verification command (`run_command`) and each bounded structured change (`apply_change`, given a
  `ChangeRequest` with the absolute target root, change spec and the run's record path). The runner still decides
  what runs, in which order, and what each status means.
- `ExperimentRunner::run` is unchanged: it runs on `Standalone`, this crate's own host (commands through XCCUTE's
  argv-safe runtime contract, bounded changes through Code Passenger's declared transform line in-process).
  `ExperimentRunner::run_with(spec, host)` runs on another host.
- A host's change error stops the run before the candidate phase, as a Code Passenger failure did.
- Tests: `tests/host.rs` (every command and change through a host that runs nothing itself; a refused change ends
  the run before the candidate).

## 0.3.1 (2026-11-01)

- Adds a pre-Custodian qualification suite covering rich cross-format read/write round-trips, schema rejection and conditional requirements, result codecs, runner phase ordering, measurement parsers, fail-fast and cleanup behavior, PathGen run layout, CLI exit semantics, optional-feature refusal contracts, and real Code Passenger success/failure transforms on dependency-free Rust fixture workspaces.
- Validates `experiments.result.v3` on result reads so stale or foreign result documents are refused just like stale experiment specs.
- Restores strict `clippy -- -D warnings`, adds rustdoc warnings-as-errors, and tests no-default, Differences-only, Code-Passenger-only, and all-feature builds before trusted publishing.

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
