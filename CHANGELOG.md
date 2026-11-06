# Changelog

## 0.5.0 (unreleased)

Exact tree equality as a measurement. The BFCB ordered history judged each replay by a measurement that ran
`sh -c` to walk up to the set, then `cargo run --bin bfcb-state-verify` over the specimen and a kept tree, with
`exit_success` as the value: a shell, a build and a second reader of trees for what is one fact, that two trees hold
the same paths with the same bytes and modes. Now an experiment says that fact and its host reads it. Minor, not
patch: `MeasurementSpec`'s `command` and `parser` are now its `source`, and `ExperimentHost` has a new method, so a
struct literal or a host written against 0.4 does not build. Every 0.4 spec reads, and writes, as it did. The schema
word stays `experiments.spec.v3`: the readers refuse only another word ([codec.rs L51](https://github.com/jameshuntt/experiments/blob/e7f17c3/src/codec.rs#L51)), 0.4.0
added `env` under v3, and a 0.4 reader given an exact tree measurement refuses it (no `command`) rather than misread
it. Links are pinned to 35f68e9.

- **A measurement is one or the other** ([`MeasurementSpec`](https://github.com/jameshuntt/experiments/blob/35f68e9/src/model.rs#L80), was
  [`command` and `parser`](https://github.com/jameshuntt/experiments/blob/e7f17c3/src/model.rs#L79)): its [`source`](https://github.com/jameshuntt/experiments/blob/35f68e9/src/model.rs#L93) is a command with its
  parser, or an [`ExactTree`](https://github.com/jameshuntt/experiments/blob/35f68e9/src/model.rs#L108) `{ path, equals_tree }`. Every format reads and writes one form
  ([`MeasurementForm`](https://github.com/jameshuntt/experiments/blob/35f68e9/src/model.rs#L118)): `command` and `parser` as before, or `exact_tree`; both, neither, a
  command without its parser, a parser with no command are each refused as read, saying which
  ([L132](https://github.com/jameshuntt/experiments/blob/35f68e9/src/model.rs#L132)). The 1o schema takes `exact_tree` and requires `phase`, `condition` and `name` (the
  rest is the reader's rule, which every format shares).
- **Where the trees are.** `path` is where a command runs (the target root); `equals_tree` is where a change spec is,
  by the one rule the runner resolves both by ([`declared`](https://github.com/jameshuntt/experiments/blob/35f68e9/src/mutation.rs#L115), used for the change at
  [L31](https://github.com/jameshuntt/experiments/blob/35f68e9/src/mutation.rs#L31), was [inline](https://github.com/jameshuntt/experiments/blob/e7f17c3/src/mutation.rs#L31), and for the tree at
  [runner.rs L330](https://github.com/jameshuntt/experiments/blob/35f68e9/src/runner.rs#L330)); a campaign's binding resolves it against the declared target, as it
  resolves each change spec ([bind.rs L62](https://github.com/jameshuntt/experiments/blob/35f68e9/src/campaign/bind.rs#L62)).
- **The host compares** ([`compare_exact_tree`](https://github.com/jameshuntt/experiments/blob/35f68e9/src/runner.rs#L80), given an
  [`ExactTreeRequest`](https://github.com/jameshuntt/experiments/blob/35f68e9/src/runner.rs#L88)): it returns an [`ExactTreeRecord`](https://github.com/jameshuntt/experiments/blob/35f68e9/src/model.rs#L358), the two
  trees, both digests, both file counts, every differing path, `equal`; `equal` is the measurement's value, unequal
  trees are a record and not an error. The result keeps every record in [`exact_trees`](https://github.com/jameshuntt/experiments/blob/35f68e9/src/model.rs#L402) (read
  as empty from a result written before; not written when empty).
- **Standalone does not compare trees** ([L143](https://github.com/jameshuntt/experiments/blob/35f68e9/src/runner.rs#L143)): the rule a tree is read by is the governing
  host's (Custodian's), never a second copy here. `ExperimentRunner::run` refuses an exact tree measurement before
  anything runs ([L164](https://github.com/jameshuntt/experiments/blob/35f68e9/src/runner.rs#L164), [`exact_tree_is_the_hosts`](https://github.com/jameshuntt/experiments/blob/35f68e9/src/runner.rs#L100)).

Tests: `tests/exact_tree.rs`: `a_measurement_is_a_command_with_its_parser_or_an_exact_tree_in_every_format` (1o, JSON,
YAML, YML, TOML round trips; both, neither, no parser, a parser and a tree, an unknown key, each refused),
`a_spec_written_before_reads_as_it_did_and_writes_the_same_keys` (a 0.4 command measurement's JSON, byte for byte; the
shipped examples), `the_standalone_host_refuses_an_exact_tree_measurement_before_anything_runs` (no prepare ran,
nothing stored), `a_hosts_exact_tree_record_is_the_measurement_and_the_result_keeps_it` (equal and not:
the value, the promotion, the record read back), `a_campaigns_binding_resolves_equals_tree_where_a_change_spec_is_resolved`.

`cargo test --all-features`: 63 passed, 0 failed (5 of them in `tests/exact_tree.rs`).

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

What a review found, and its fixes (links pinned to fb3c1cd):

- **`from` a sequence that ends in many states** resolved to the node before its last step. A sequence now ends
  where its last step ends ([plan.rs L721](https://github.com/jameshuntt/experiments/blob/fb3c1cd/src/campaign/plan.rs#L721)); its lineage goes on through a parallel inside it
  ([plan.rs L403](https://github.com/jameshuntt/experiments/blob/fb3c1cd/src/campaign/plan.rs#L403)), but `from` it is refused when its last step is a parallel, repeat or sweep
  ([plan.rs L997](https://github.com/jameshuntt/experiments/blob/fb3c1cd/src/campaign/plan.rs#L997)). [`from_a_sequence_that_ends_in_many_states_is_refused`](https://github.com/jameshuntt/experiments/blob/fb3c1cd/tests/campaign.rs#L764).
- **An undeclared change accepted where another node starts from it** (exploratory, shared). It is never accepted in
  the shared workspace, a `continue` lineage, or an isolated node another starts from; exploratory keeps it only in a
  disposable isolated workspace ([run.rs L445](https://github.com/jameshuntt/experiments/blob/fb3c1cd/src/campaign/run.rs#L445), `PlanNode::disposable` [plan.rs L1092](https://github.com/jameshuntt/experiments/blob/fb3c1cd/src/campaign/plan.rs#L1092)).
  [`exploratory_keeps_an_undeclared_change_only_where_nothing_starts_from_it`](https://github.com/jameshuntt/experiments/blob/fb3c1cd/tests/campaign.rs#L801).
- **Determinism.** Nodes are ordered dependencies first, then by id ([plan.rs L1030](https://github.com/jameshuntt/experiments/blob/fb3c1cd/src/campaign/plan.rs#L1030)), not by where a
  document lists them; sweep trials are `<id>[.bNN].tNN.rNN`, the expansion's order; groups by id. Experiment paths are
  resolved lexically (`sub/../a.json` is `a.json`, [plan.rs L286](https://github.com/jameshuntt/experiments/blob/fb3c1cd/src/campaign/plan.rs#L286)) and must stay inside the campaign's
  folder ([plan.rs L302](https://github.com/jameshuntt/experiments/blob/fb3c1cd/src/campaign/plan.rs#L302)). [`equivalent_documents_make_one_plan_whatever_order_and_spelling`](https://github.com/jameshuntt/experiments/blob/fb3c1cd/tests/campaign.rs#L774).
- **Levels and excludes.** A text level holding a NUL is refused ([plan.rs L559](https://github.com/jameshuntt/experiments/blob/fb3c1cd/src/campaign/plan.rs#L559)); an exclude that is
  the whole root (`.`) is refused ([plan.rs L566](https://github.com/jameshuntt/experiments/blob/fb3c1cd/src/campaign/plan.rs#L566)).
  [`a_level_no_environment_can_carry_and_an_exclude_of_everything_are_refused`](https://github.com/jameshuntt/experiments/blob/fb3c1cd/tests/campaign.rs#L790).
- **Summaries pooled across blocks.** Every summary is one treatment in one block combination from one starting state
  (`start_digest`, [result.rs L335](https://github.com/jameshuntt/experiments/blob/fb3c1cd/src/campaign/result.rs#L335)); a factor level's marginal is within one block combination and one
  starting state ([result.rs L384](https://github.com/jameshuntt/experiments/blob/fb3c1cd/src/campaign/result.rs#L384)); nothing is pooled across either.
- **A discarded workspace** has no accepted state; `left` is the state it was left in, as read ([run.rs L134](https://github.com/jameshuntt/experiments/blob/fb3c1cd/src/campaign/run.rs#L134)).
- **A stop** names its node and exactly the nodes running then (`StopRecord`, [run.rs L182](https://github.com/jameshuntt/experiments/blob/fb3c1cd/src/campaign/run.rs#L182),
  [run.rs L503](https://github.com/jameshuntt/experiments/blob/fb3c1cd/src/campaign/run.rs#L503)); every node not started is skipped by policy, and says so.
  [`a_stop_names_the_nodes_that_were_running_and_skips_the_rest_by_policy`](https://github.com/jameshuntt/experiments/blob/fb3c1cd/tests/campaign.rs#L815).
- **A resume** keeps each earlier attempt (refused, failed, skipped, interrupted) with its result
  ([run.rs L295](https://github.com/jameshuntt/experiments/blob/fb3c1cd/src/campaign/run.rs#L295), [result.rs L215](https://github.com/jameshuntt/experiments/blob/fb3c1cd/src/campaign/result.rs#L215)); the progress keeps the tools the run began with
  ([run.rs L217](https://github.com/jameshuntt/experiments/blob/fb3c1cd/src/campaign/run.rs#L217)).
- The planner's lookups are maps and its order a queue (40000 trials no longer quadratic).
- **CI.** `consumers` tests custodian only when `cargo metadata` resolves its experiments to this checkout (no registry
  source, this manifest), else says so ([ci.yml L114](https://github.com/jameshuntt/experiments/blob/fb3c1cd/.github/workflows/ci.yml#L114)); publish waits for it
  ([ci.yml L123](https://github.com/jameshuntt/experiments/blob/fb3c1cd/.github/workflows/ci.yml#L123)).

Tests: `tests/campaign.rs` (25: formats and refusals at every depth, sequence ≡ flat, equivalent orders and spellings,
fifty ordered nodes, parallel, repeat, the 240-trial factorial, explicit ≡ factorial assignments,
one-factor-at-a-time, blocks, 39 refusals, strict and controlled, disposable workspaces, the binding through a real run,
decisions, stops, resume and attempts, summaries); `tests/formats.rs` round-trips `env` through 1o, JSON, TOML, YAML
and YML. `cargo test --all-features`: 58 passed, 0 failed (stable and Rust 1.90);
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
