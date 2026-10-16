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
the existing DSL (`id`/`ref=X`, `ref=>X`, `o`, `ro`, `mu`, `ordered`, `k`, `unset`, `unique`, `blocks-removal`,
`enum=...`, scalar types) — no
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
not be taken off again. The legacy `/caps-man/datapath` is alike: `bridge=""` is ignored,
`vlan-id=""` refused, `!bridge !vlan-id !vlan-mode` clear them. Not every field can be unset
this way (`!comment` is accepted but leaves the comment), so it is only on the fields where it
was checked on a CHR.

`sort_mutations` orders the mutations from what each one needs and takes, besides the names it
refers to (`depends`) and makes (`provides`). An update depends only on the references it writes
(`written_references`): a `set` sends the changed values only, so a kept reference is there
already and an update without changes needs nothing (a bound radio reports the bridge of its
configuration's datapath as `datapath.bridge`, which needn't be in the target).
`ResourceMutation::ordering` (`MutationOrdering`)
holds the names it gives up (`releases`: a removal, a rename), the `unique` values it claims and
frees, the `blocks-removal` references it drops, and the name its `[find ...]` looks for
(`finds`, `KeyedResource::key_reference`). A mutation runs after the ones still holding a name it
provides or a value it claims, and a name is given up only after the mutations that drop a
blocking reference to it or find by it. Mutations waiting on each other (two VXLANs swapping their
VNIs) are a `MissingDependenciesError` without missing dependencies, not a script that fails.
- `unique`: no two entries of the menu hold the value; `vni` of `/interface/vxlan` (7.24.5
  refuses a second VXLAN with a VNI, regardless of the port). A VXLAN renamed while keeping its
  VNI is removed before the new one is added; a new VNI under the same name is set in place.
- `blocks-removal`: the referenced entry must not be removed while the reference is set, as
  RouterOS refuses it (a radio's `datapath`: `set !datapath` before the datapath goes) or removes
  the referencing entry with it (a vtep's `interface`: the vtep is removed before its VXLAN, not
  after, when it is gone). A wifi configuration's `datapath` is marked as well, unchecked: a
  CAP's radios are bound through it, so it doesn't point to nothing in between. Other
  references don't hold an entry back: removing a VXLAN, VLAN,
  bridge, list, routing table or OSPF area/instance leaves them dangling as `*id` (7.24.5), and
  `set ... interface=new` mends them. A `by_id` entry isn't moved to another entry a blocking
  reference names (a vtep to another VXLAN) but replaced, as the move would wait for the new
  VXLAN, which may wait for the old one to go.

`default=<value>` gives a required field the value RouterOS leaves out: a firewall rule added
without `disabled` doesn't report it (`default=false`), nor does the OSPF instance an update from
RouterOS 6 brings report `vrf` (`default=main`). A dhcp-client with
`add-default-route=no` hides `default-route-distance` and refuses to clear it (`=""`,
`!default-route-distance`), so it is required with RouterOS' own value (`default=1`, what a new
client gets; the manual names none): the target sets 1 there, which is accepted and then
matches. Match fields of the firewall (ports, protocol,
addresses, limits) are plain text, not typed: RouterOS reports ranges, lists, negations and
`value:mode` pairs there (`33434-33534`, `!fe80::/10`, `in,ipsec`, `equal:1`, protocol `139`).
`ref=X` provides a name, `ref=>X` uses one; a field that names interfaces (`/interface/wireless/cap
interfaces`) must be `ref=>interface`, or it isn't ordered after the rename it refers to.

`mu` is a `BTreeSet` (order doesn't count); `mu; ordered` a `Box<[V]>` for lists RouterOS keeps in
order and where it matters (`dns-server`, `/ip dns servers`, `/ipv6 nd dns`: the first is asked
first). Not for references (a rename can't keep the position). The element type needs
`value::OrderedElement` (IP addresses so far; `Box<[u8]>` is a raw text value, so the impl can't
be for every `RosValue`).

An entity with a read-only `dynamic` field is fetched without its dynamic entries (RouterOS
makes them itself), except where the key is read-only hardware (`id=default-name`): the
radios of a CAP that CAPsMAN manages are flagged dynamic (`/interface/wifi`, 7.23.7), its
virtual slaves too, but only the radios have a `default-name`.

`can-add` says the menu adds entries (`/interface/wifi` adds virtual interfaces). A
`by_key` field on a read-only key (`default-name`) still only updates: what has that key is
hardware, it can't be added or removed, so the macro doesn't ask for `Creatable` there.
`reuse` marks a menu with an entry of RouterOS's own that it refuses to remove but whose key
may be changed (`/ipv6/nd`'s first, `default=yes`, `can not remove default rule`): an entry the
target no longer has takes one it adds, in the device's order, by a `set` of the key
(`generate_add_update_remove_by_key_reusing`).

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
