# experiments

`experiments` is the reproducible execution layer above `scientific-method`.

An experiment is a declarative `.1o` document. The instrument executes its
declared protocol, collects baseline and candidate measurements at the correct
experimental points, records verification and Differences evidence, asks
`scientific-method` what the observations mean, and records a separate
promotion decision.

## Contract stack

- `simple_impl_derive`: declares the `experiments` CLI once.
- `xccute_contract`: supplies the generated argv/validation/help/status contract.
- `xccute_runtime`: receives dynamic experiment operations as argv-safe
  `CommandSpec` values; experiments never builds raw shell strings.
- `scientific-method`: scientific vocabulary and criterion evaluation only.
- `pathgen`: owns experiment data/state roots.
- `differences`: answered through its library's declared simple_impl/xccute
  surface; experiments does not spawn the `differences` binary.
- `experiments`: spec → protocol → phase-correct observations → evaluation →
  result/promotion.

## Important separation

Verification, scientific support, and promotion are different decisions:

1. verification says whether the candidate satisfies declared validity checks;
2. `scientific-method` evaluates the declared evidence;
3. promotion applies the experiment's separate retention policy.

Passing tests never implies scientific support by itself.

## Run

```sh
experiments run examples/minimal.1o
```

Optional PathGen development base:

```sh
experiments run examples/minimal.1o .experiment-storage
```

Exit status is declared on the typed command surface:

- `0` promote
- `1` refused/error
- `2` do not promote
- `3` inconclusive

## Custodian

Custodian-specific execution is intentionally not implemented in this release.
The generic experiment/CLI/XCCUTE/Differences contracts are established first;
the Custodian adapter can then consume them without redefining experiment
semantics.

## License

MIT OR Apache-2.0
