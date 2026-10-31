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
`build`, see `src/model/test/bgp.rs`) is the next-best substitute — it still catches field-type
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
`vlan-id=""` refused, `!bridge !vlan-id !vlan-mode` clear them. `/interface/wifi/datapath`
refuses `""` on all its fields but `bridge` (taken as `none`) and clears them all with `!field`
(7.24.5; `vlan-id=""` stopped an apply on router-stall-burri). The wifi configuration and
radio refuse `""` on their references with two of a kind ("ambiguous value", `security`,
`channel`, `aaa`, `interworking`, `steering`, `datapath.interface-list`) and on every number,
flag, duration and choice of their groups (`security.connect-priority`). A sweep of every
optional field of the modeled menus on a CHR 7.24.5 (`set` on an empty `[find]` validates the
value without changing anything) found more refusing `""`; where `!field` cleared the value of a
real entry (set, then `!field`, then read back empty) they are `unset` too: the match fields of
`/ip/firewall/filter`, `nat` and `/ipv6/firewall/filter` (a defconf rule's `in-interface-list`,
paired with a rule without one), the legacy CAPsMAN configuration, datapath and security, the
wifi provisioning's `master-configuration` and `radio-mac`, the OSPF area's `default-cost` and
`nssa-translator`, the instance's `domain-tag`, `originate-default`, `use-dn`, and the VXLAN's
`interface` and `group` (only together). The PoE fields of `/interface/ethernet` are missing on
a CHR: `power-cycle-ping-address=""` stopped an apply on lab-np2 (CRS318, 7.23.8, "invalid value
for argument ip"), so it and `power-cycle-ping-timeout` are `unset` on the strength of `!field`
clearing an address in a CHR script (`local-address` of a VXLAN). Not every field can be unset this
way (`!comment` is accepted but leaves the comment), so it is only on the fields where it was
checked on a CHR. Fields that take neither (`horizon`, the bridge's multicast settings, most of
`/ip/dhcp-server`) need a value in the target or `default=`. The `hw` of a bridge port is `k`: only
a radio's port has none in the target (RouterOS doesn't report it there), and a port taken over for
one refused `hw=""` (lab-wap, 7.23.8).
`/ipv6/dhcp-client` is keyed by `interface`: RouterOS takes one client per interface ("dhcp-client
on that interface already exists", CHR 7.24.5). It reports `pool-prefix-length` only once set,
and `default-route-distance` only with a default route, so both are `o`.
The static DHCP leases (`/ip/dhcp-server/lease`, keyed by `address`) are such a menu (CHR
7.24.5, the notes in `DhcpServer.txt`): the flags refuse `""` and are reported only when on
(`default=false`, cleared with `no`), `lease-time` takes `0s`, not `""`, `parent-queue=""`
reads back as `none` (`none; default=none`), `queue-type` and `insert-queue-before` take only
`!field`, and `server=""` is ambiguous while `server=all` drops the field, so `server` is
optional and a target always names one. A lease by client ID has no `mac-address`.

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
blocking reference to it or find by it. `/ipv6/nd` is keyed by its `interface`, a `ref=>interface`: found
after its VLAN interface was removed, `[find interface=vlan1-lan]` finds nothing (the entry names
`*30` then, CHR 7.24.5) and the `set` does nothing without an error (1og-sw1 sent no router
advertisement after the apply). The own entry's `all` is RouterOS', a seed of the sort. Mutations waiting on each other (two VXLANs swapping their
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

`by_id(..., owned = "<prefix>")` is for a menu shared with entries of others (the firewall):
only the entries whose `comment` starts with the prefix are the target's
(`generate_owned_by_id`); the others are neither changed nor removed, except that one without a
comment equal to a target but for the comment is taken over by setting it. With `ordered` the
own entries come first, in the target's order: a `ResourceMutationOperation::Move` (`move
[find comment=…] destination=<.id>`, before the first other entry, or to the end without one)
puts them there where the device's order isn't that already, or anything was added (an `add`
goes to the end). An own entry whose comment goes on after the prefix with `last `
(`OWNED_LAST`) comes after all others instead, by a second move to the end: the drop that ends a
firewall chain, after the rules of others (set up by hand) had their say. The comments must be
unique in the menu, as the move finds by them; a move waits for every other mutation of its menu
(`waits_for`), the one to the end (`Move { last: true }`) for the other move too. `move X destination=Y` puts X before
Y, without `destination` at the end (CHR 7.24.5).

`default=<value>` gives a required field the value RouterOS leaves out: a firewall rule added
without `disabled` doesn't report it (`default=false`), nor one added without `action`
(`default=accept` in `/ip/firewall/filter`, `nat` and `/ipv6/firewall/filter`; the other IPv6
tables are unrefined and unused), nor does an entry of the wifi menus
(`aaa`, `channel`, `configuration`, `datapath`, `interworking`, `provisioning`, `security`,
`steering`; 7.24.5, `multi-passphrase` does), nor does the OSPF instance an update from
RouterOS 6 brings report `vrf` (`default=main`). A dhcp-client with
`add-default-route=no` hides `default-route-distance` and refuses to clear it (`=""`,
`!default-route-distance`), so it is required with RouterOS' own value (`default=1`, what a new
client gets; the manual names none): the target sets 1 there, which is accepted and then
matches. A bridge likewise hides its STP settings without STP (`priority`, `forward-delay`,
`max-message-age`, `transmit-hold-count`) and the MSTP ones without MSTP (`max-hops`,
`region-revision`), keeps them and rejects an empty value (`forward-delay=""`, 7.24.5), so they
are required with RouterOS's defaults. An OSPF interface template left from an older configuration (inactive, a gone
interface, `networks=`; 7.23.8 after an update from 7.21.5) reports only `area`, `interfaces`,
`networks` and `priority`, so its cost, intervals, `instance-id`, `type` and `disabled` have
RouterOS's values as `default=` (`src/model/test/ospf.rs`; a CHR 7.24.5 reports them all, even for such an
entry, and `!cost` doesn't take them away). Match fields of the firewall (ports, protocol,
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
virtual slaves too, but only the radios have a `default-name`. The exception follows the header's
`id=`, not the read-only key alone: `/ip/service` has a read-only `name` (set, never added) but no
`id=name`, so its dynamic entries (open connections, `socks`, `btest`) are left out. Where they are
left out, a `[find ...]` of the script looks only at the static ones too (`dynamic=no`,
`model::finds_static_only`, likewise the API's `?dynamic=false`): `[find name=api]` also found
the open API connection, and RouterOS refused the `set` with "this is configured elsewhere
(/ip/service/set *0 = telnet)" (lab-wap 7.23.8, CHR 7.24.5).

`can-add` says the menu adds entries (`/interface/wifi` adds virtual interfaces). A
`by_key` field on a read-only key (`default-name`) still only updates: what has that key is
hardware, it can't be added or removed, so the macro doesn't ask for `Creatable` there.
`reuse` marks a menu with an entry of RouterOS's own that it refuses to remove but whose key
may be changed (`/ipv6/nd`'s first, `default=yes`, `can not remove default rule`): an entry the
target no longer has takes one it adds, in the device's order, by a `set` of the key
(`generate_add_update_remove_by_key_reusing`).

`/system/logging` and `/system/logging/action` hold entries of RouterOS's own (`default`) that it
refuses to remove; the target has to carry them. An action reports only the fields of its
`target` and of its `remote-log-format`: `syslog-time-format` and `cef-event-delimiter` only in
CEF, where RouterOS refuses to clear them (`=""` is refused, `!field` is a syntax error; CHR
7.24.5), so they are plain `o` and the target keeps the action in CEF. An empty delimiter is
reported as none. The formats are `default`, `syslog` and `cef` (ap-2og-aussen, 7.23.8, had
`syslog`; `bsd-syslog` is a `syslog-time-format`); `syslog` reports `syslog-facility`,
`syslog-severity` and `add-topics-string` besides, read-only here as the target is CEF, where
they are hidden. `prefix=""` and `regex=""` of a rule clear them.

`InterfaceBridgeCfg::snoop_multicast` turns IGMP/MLD snooping on; the querier stays off. The
multicast settings are required with RouterOS's values (`default=`): with snooping it reports
them, without it hides and keeps them and refuses to clear them (`igmp-version=""` is
ambiguous; CHR 7.24.5, ap-klr-solar 7.23.7), so turning snooping off sets only `igmp-snooping`.

A field reported twice in one answer counts once, with its first value (`first_of_each`): a
bridge port of a bridge with IGMP snooping reports `multicast-router` as its setting and further
on as its state (`false`, lab-core 7.24.5), which was a diff forever.

`apply_mutations_via_script` runs its script with `SCRIPT_RUN_TIMEOUT` (10 min) instead of
mikrotik-api's 120 s without a reply: `/system/script/run` answers only at the script's end.

`mikrotik-model-generator/build.rs` emits `cargo:rerun-if-changed=ros_model` — without it, editing
or adding a `ros_model/*.txt` file does not invalidate cargo's build cache (`include_dir!` doesn't
register per-file dependencies on stable Rust), so `cargo check`/`build` can silently keep using
stale generated code that doesn't reflect the change at all. Don't remove that build script.

Tests of the generated model go in `src/model/test/`, one file per menu or topic, declared in
its `mod.rs`; not as `*_tests.rs` next to the model.

## Commits

- One focused commit per logical change; split unrelated work (e.g. a refactor and the feature
  built on it) into separate commits.
- Subject: short, lowercase, imperative, no trailing period (e.g. `add bgp connection entity`).
- Body: why the change was needed and anything non-obvious, wrapped at ~72 columns.
- Update this file in the same commit as the change it describes.
- Commits by Claude end with the `Co-Authored-By: Claude …` trailer, without a session link.
- Push to `master` only when asked.
- Verify with `cargo clippy --all-targets` in addition to `cargo check`/`cargo test`, here and
  in `mikrotik-model-generator/` (its tests and lints don't run from the crate root). Both are
  without warnings, the generated code included: a generator change that brings one back
  should emit code clippy accepts, not an `allow`. The generated code is the same on every
  build (`generates_the_same_code_each_time`): what it iterates is ordered (`BTreeMap`), not a
  `HashMap`.
