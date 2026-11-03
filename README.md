# experiments

`experiments` is the reproducible execution layer above `scientific-method`.
It owns experiment documents and execution; `scientific-method` remains the
pure vocabulary and evaluator.

## Experiment documents

One experiment model is readable in four formats:

- real 1o, parsed by `core-1o` and validated against the shipped experiment schema;
- JSON;
- TOML;
- YAML/YML.

The canonical 1o filename is `v1.<name>.1o.experiment`. The crate ships the
schema in the normal `core-1o` workspace layout:

```text
schema/
  1o.version.json
  experiment/
    v1.schema.json
```

That directory is available to `core-1o` workspace tooling and editors. The
same schema file is compiled into the crate for standalone CLI validation, so
an installed `experiments` binary does not depend on a source checkout to
validate a 1o experiment.

The examples are equivalent documents:

```text
examples/v1.minimal-latency.1o.experiment
examples/minimal-latency.json
examples/minimal-latency.toml
examples/minimal-latency.yaml
```

The 1o example is actual 1o syntax, not JSON with a different suffix:

```text
experiment {
  schema "experiments.spec.v3"
  id "minimal-latency"

  scientific_method {
    hypothesis {
      statement "the candidate lowers latency"
    }
    criterion {
      kind "improvement"
      baseline { condition "baseline" measurement "latency" }
      candidate { condition "candidate" measurement "latency" }
      direction "lower_is_better"
      minimum_fraction 0.05
    }
  }

  target { root "." }
  mutation { mode "unbounded" backend "code_passenger" }
}
```

## Execution stack

- `simple_impl_derive` declares the `experiments` CLI once.
- `xccute_contract` supplies argv, validation, help, display, and status semantics.
- `xccute_runtime` executes declared dynamic protocol operations without shell-string construction.
- `scientific-method` evaluates observations as `supported`, `not_supported`, or `inconclusive`.
- `pathgen` owns experiment data/state storage.
- `differences` is called through its typed library surface, not by spawning its CLI.
- `code_passenger` is the standalone structured source-mutation backend.

## Mutation modes

`mutation.mode = "bounded"` means source intervention is restricted to declared
`mutation.changes`. Each change names a Code Passenger `ChangeSpec` (1o or
TOML). `experiments` builds Code Passenger's own typed `transform` line and
runs it in-process. Code Passenger plans and proves the edit on a scratch copy,
writes it, checkpoints the tree, and produces a transform record that is kept
as experiment evidence.

In bounded mode, `protocol.intervention` commands are refused. This prevents an
experiment from claiming bounded mutation while also running an arbitrary edit
command.

`mutation.mode = "unbounded"` permits the explicitly declared XCCUTE
`protocol.intervention` operations. Structured Code Passenger changes may still
be included when useful.

PathGen is storage/layout; it is not a source-code replacement engine. Code
Passenger owns standalone structured writes. A later Custodian adapter can plug
into the same mutation boundary without redefining experiment semantics.

Example bounded mutation fragment:

```text
mutation {
  mode "bounded"
  backend "code_passenger"
  change {
    label "add candidate field"
    spec "changes/add-candidate-field.1o.change"
  }
}
```

## Scientific separation

Verification, scientific support, and promotion are separate decisions:

1. verification says whether the candidate satisfies declared validity checks;
2. `scientific-method` evaluates the declared observations and criteria;
3. promotion applies the experiment's separate retention policy.

Passing tests never implies scientific support by itself.

## Run

```sh
experiments run examples/v1.minimal-latency.1o.experiment
```

Optional PathGen development base:

```sh
experiments run examples/v1.minimal-latency.1o.experiment .experiment-storage
```

Run results are deliberately written as `result.json`; the crate does not put
JSON behind a `.1o` filename. Explicit result import/export supports JSON,
TOML, and YAML. A 1o result format will require its own result schema rather
than pretending the experiment-spec schema covers results.

Exit status is declared on the typed command surface:

- `0` promote
- `1` refused/error
- `2` do not promote
- `3` inconclusive

## Qualification before Custodian

The release gate deliberately proves the standalone boundary before Custodian consumes it:

- rich experiment specs round-trip through real 1o, JSON, TOML, YAML and YML without semantic drift;
- the shipped 1o schema is discovered by `core-1o` and rejects unknown keys, wrong types, missing required keys and conditional criterion omissions;
- runner tests prove baseline measurement precedes intervention, candidate measurement follows it, bounded mode refuses raw intervention before prepare, unbounded mode executes declared XCCUTE intervention, all measurement parsers record the expected scalar, cleanup failure remains evidence, and fail-fast phases stop later work;
- PathGen run/result/evidence/receipt/difference directories are asserted from a real run;
- CLI tests lock exit 0/1/2/3 to promote/error/do-not-promote/inconclusive;
- Code Passenger is exercised end-to-end on a dependency-free Rust fixture: a real `make_struct` ChangeSpec is cargo-proved, written, checkpointed and returned as experiment evidence, while a deliberately uncompilable transform is proved to leave the specimen unchanged;
- CI tests all features on stable and Rust 1.90, plus no-default, Differences-only and Code-Passenger-only shapes, with `clippy -D warnings`, rustdoc warnings-as-errors, package integrity and trusted publishing.

## Campaigns

A campaign composes experiments over an evolving accepted state, and expands
experimental designs over factors. Its document (`CampaignSpec`, schema
`experiments.campaign.v1`) is real 1o (`v1.<name>.1o.campaign`, validated at
every depth of its composition against `schema/campaign/v1.schema.json`) or its
JSON, TOML, YAML or YML twin; every format refuses a key the campaign does not
declare.

```text
campaign {
  id "history"
  sequence {
    run { id "A" spec "a.json" }
    run { id "B" spec "b.json" }
  }
  repeat { id "again" times 10 from "B" run { id "b" spec "b.json" } }
  sweep { id "grid" spec "bench.json" design "factorial" factor "matcher" factor "population" repeats 10 }
}
```

- `run`: one experiment, with its factor levels (`set`) and blocks; at the top
  level it may name `after` (nodes that finish first) and `from` (the one node
  whose accepted state it starts from) and its `workspace` (`shared`,
  `isolated`, `continue`): the flat form.
- `sequence`: each step starts from the accepted state the one before left.
- `parallel`: branches from one state, each in its own isolated workspace, no merge.
- `repeat`: N independent trials of one treatment, each isolated, from one state.
- `sweep`: a design over one experiment: `factorial` (every combination),
  `one_factor_at_a_time` (each factor moved alone from its `reference`),
  `explicit` (named `treatment`s), each treatment repeated, crossed with blocks.
- `factor` (an independent variable: an experiment's own, or scope `campaign`),
  `control` (a value held fixed), `block` (a nuisance variable, never a treatment).
- `rigor`: `exploratory`, `controlled` (default), `strict`; `policy`:
  `on_findings`, `on_refusal`, `on_error`, `on_undecided`, `max_parallel`.

`LoadedCampaign::load` makes one deterministic `CampaignPlan` before anything
runs, or refuses it with every reason (duplicate ids, missing files and
references, cycles, ambiguous state parents, two writers on one workspace,
zero repeats, unknown factors and levels, bad blocks and controls, impossible
designs, unbounded mutation in the shared workspace, what strict cannot
establish). A sequence and its flat form are one plan with one digest.

A trial's assignment reaches its experiment as environment variables every
command is given through `CommandSpec::env` (`EXPERIMENT_FACTOR_<NAME>`,
`EXPERIMENT_CONTROL_<NAME>`, `EXPERIMENT_BLOCK_<NAME>`, `EXPERIMENT_TRIAL`,
`EXPERIMENT_REPLICATE`, `EXPERIMENT_TREATMENT`, `EXPERIMENT_CAMPAIGN`): typed
levels in one text form, no templating, no shell. Each command's record keeps
what it was given.

`CampaignRunner` decides what a node's execution makes of its candidate
(promotion, the undecided policy, an undeclared change by rigor), applies the
stop policies, keeps the progress a resume reads (refused when anything the run
was made from moved), and makes the `CampaignResult`
(`experiments.campaign-result.v1`): every node as planned and as it ran, every
state digest, every result's path and digest, the order things ran in, and the
group summaries (counts; n, mean, median, min, max, sample standard deviation of
each numeric measurement), with no significance claimed.

A campaign runs only under a host that governs the workspaces: Custodian's
`custodian experiment campaign CAMPAIGN [STORAGE] [--job JOB] [--plan] [--again]`.
`examples/v1.matrix.1o.campaign` is a 2 × 4 × 3 factorial with ten repeats
(240 trials) over `examples/matrix-bench.json`.

## Custodian

Custodian hosts an experiment (`custodian experiment run`) through this crate's
`ExperimentHost`, and governs a campaign (`custodian experiment campaign`):
its workspaces, its writer, its receipts, every source change made by Code
Passenger through Custodian's preflight and transform lines.

## License

MIT OR Apache-2.0
