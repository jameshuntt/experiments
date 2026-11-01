# experiments

`experiments` is the reproducible execution layer above `scientific-method`.

An experiment is a declarative `.1o` file. The instrument executes its declared protocol, collects measurements, records verification and Differences evidence, asks `scientific-method` what the observations mean, and records a separate promotion decision.

The separation is intentional:

- tests/verification say whether the candidate is valid enough to measure;
- scientific criteria say whether the hypothesis is supported;
- promotion says whether the candidate should be retained/promoted.

## Layers

- `scientific-method`: scientific vocabulary and criterion evaluation only.
- `pathgen`: owns the experiment instrument's data/state roots.
- `differences`: optional external evidence source (`differences <kind> ...`).
- `custodian`: optional embedded seam exposing `custodian_kit` and `differences_kit` side by side.
- `experiments`: spec → run → observations/evidence → result.

## Run

```sh
cargo run -- run examples/minimal.1o
```

A successful promoted experiment exits 0. A measured experiment that should not promote exits 2. An inconclusive experiment exits 3.

## Storage

All storage roots come from PathGen. Subpaths are named in one place (`ExperimentPaths`) and include specs, runs, results, evidence, receipts, and differences.

## License

MIT OR Apache-2.0
