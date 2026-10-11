# mikrotik-model

Typed RouterOS configuration model, generated at build time from declarative `.txt` definition
files under `mikrotik-model-generator/ros_model/{interface,ip,ipv6,routing,system,caps-man}/*.txt`
(plus `enums.yaml`), via the `mikrotik_model!` macro (`mikrotik-model-generator-macro`). Builds on
`mikrotik-api` (in this workspace: `../mikrotik-api`) for the transport. Provides, per modeled
RouterOS path, both a `{Name}Current` fetcher and a `{Name}Target` diff/mutation generator.

`inspect-device` (binary in this crate) connects to a live device and dumps its discovered
config/model — useful for validating new `ros_model/*.txt` entity definitions against real
firmware before treating them as final. When no live device is reachable, a static decode test
(feed real `/export` key=value pairs straight into the generated `*Builder`'s `append_field`/
`build`, see `src/model/bgp_tests.rs`) is the next-best substitute — it still catches field-type
mismatches (e.g. an address field that needs to accept a RouterOS `%zone`-suffixed value) without
needing network access.

Adding a new RouterOS config path is normally just a new `.txt` file in `ros_model/`, following
the existing DSL (`id`/`ref=X`, `ref=>X`, `o`, `ro`, `mu`, `ordered`, `k`, `unset`, `enum=...`, scalar types) — no
generator source changes needed for a simple addition.

`k` (keep-if-none: a `None` target leaves the device's value alone) is only for fields where the
existing value can't falsify the result, e.g. a hardware `mac-address`. A field that is merely
missing from some devices' output or that the generator doesn't care about is plain `o`, and the
target then sets it explicitly. Note that a `None` target clears the field, which RouterOS rejects
for booleans (`mvrp=`), so such a field needs a concrete value in `defaults.rs` or the generator.
A field RouterOS accepts but never reports in some state (a bridge without vlan filtering hides
`ether-type`, `frame-types`, `ingress-filtering`, `pvid`, and without mstp `max-hops`,
`region-revision`) needs a `None` target in that state, or it is a diff forever. `mvrp` on a
bridge is `k`: without vlan filtering it has no effect, and whether it is reported then depends
on the version (7.20.8 reports `false`, 7.24.5 nothing), so neither a value nor a clearing `None`
works there.

`unset` clears an optional field with a `None` target by `!field` (API `=!field=`, script
`set ... !field`) instead of `field=""`: RouterOS ignores `datapath=""` on `/interface/wifi`
without an error and refuses `datapath.bridge=""` as ambiguous (7.24.5), so a datapath could
not be taken off again. Not every field can be unset this way (`!comment` is accepted but
leaves the comment), so it is only on the fields where it was checked on a CHR.
`sort_mutations` puts a removal of something referable (`ResourceMutation::releases`) after the
updates, which may still drop references to it (`set !datapath` before the datapath goes).

`default=<value>` gives a required field the value RouterOS leaves out: a firewall rule added
without `disabled` doesn't report it (`default=false`), nor does the OSPF instance an update from
RouterOS 6 brings report `vrf` (`default=main`). Match fields of the firewall (ports, protocol,
addresses, limits) are plain text, not typed: RouterOS reports ranges, lists, negations and
`value:mode` pairs there (`33434-33534`, `!fe80::/10`, `in,ipsec`, `equal:1`, protocol `139`).

`mu` is a `BTreeSet` (order doesn't count); `mu; ordered` a `Box<[V]>` for lists RouterOS keeps in
order and where it matters (`dns-server`, `/ip dns servers`, `/ipv6 nd dns`: the first is asked
first). Not for references (a rename can't keep the position). The element type needs
`value::OrderedElement` (IP addresses so far; `Box<[u8]>` is a raw text value, so the impl can't
be for every `RosValue`).

`can-add` says the menu adds entries (`/interface/wifi` adds virtual interfaces). A
`by_key` field on a read-only key (`default-name`) still only updates: what has that key is
hardware, it can't be added or removed, so the macro doesn't ask for `Creatable` there.

`mikrotik-model-generator/build.rs` emits `cargo:rerun-if-changed=ros_model` — without it, editing
or adding a `ros_model/*.txt` file does not invalidate cargo's build cache (`include_dir!` doesn't
register per-file dependencies on stable Rust), so `cargo check`/`build` can silently keep using
stale generated code that doesn't reflect the change at all. Don't remove that build script.

## Commits

- One focused commit per logical change; split unrelated work (e.g. a refactor and the feature
  built on it) into separate commits.
- Subject: short, lowercase, imperative, no trailing period (e.g. `add bgp connection entity`).
- Body: why the change was needed and anything non-obvious, wrapped at ~72 columns.
- Update this file in the same commit as the change it describes.
- Commits by Claude end with the `Co-Authored-By: Claude …` trailer, without a session link.
- Push to `master` only when asked.
- Verify with `cargo clippy` in addition to `cargo check`/`cargo test` — not every pre-existing
  warning needs fixing, but a change shouldn't leave new, avoidable ones behind.
