# mikrotik-model

Typed RouterOS configuration model, generated at build time from declarative `.txt` definition
files under `mikrotik-model-generator/ros_model/{interface,ip,ipv6,routing,system,caps-man}/*.txt`
(plus `enums.yaml`), via the `mikrotik_model!` macro (`mikrotik-model-generator-macro`). Builds on
`mikrotik-api` (in this workspace: `../mikrotik-api`) for the transport. Provides, per modeled
RouterOS path, both a `{Name}Current` fetcher and a `{Name}Target` diff/mutation generator.

`inspect-device` (binary in this crate) connects to a live device and dumps its discovered
config/model — useful for validating new `ros_model/*.txt` entity definitions against real
firmware before treating them as final.

Adding a new RouterOS config path is normally just a new `.txt` file in `ros_model/`, following
the existing DSL (`id`/`ref=X`, `ref=>X`, `o`, `ro`, `mu`, `k`, `enum=...`, scalar types) — no
generator source changes needed for a simple addition.

## Commits

- One focused commit per logical change; split unrelated work (e.g. a refactor and the feature
  built on it) into separate commits.
- Subject: short, lowercase, imperative, no trailing period (e.g. `add bgp connection entity`).
- Body: why the change was needed and anything non-obvious, wrapped at ~72 columns.
- Update this file in the same commit as the change it describes.
- Commits by Claude end with the `Co-Authored-By: Claude …` trailer, without a session link.
- Push to `master` only when asked.
