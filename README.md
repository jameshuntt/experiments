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

## Custodian

Custodian-specific execution is intentionally not implemented in this release.
`experiments` first establishes the real document/schema layer and the
standalone Code Passenger mutation backend. Custodian can then integrate as a
thin managed backend over the same experiment contract.

## License

MIT OR Apache-2.0
