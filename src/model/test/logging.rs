//! Static decode tests for `/system/logging` and its actions (`ros_model/system/Logging.txt`),
//! with the API output of CHR 7.24.5.

use crate::model::{SystemLogging, SystemLoggingAction, SystemLoggingActionBuilder, SystemLoggingBuilder};
use crate::resource::{AppendFieldResult, DeserializeRosBuilder, DeserializeRosResource};

fn build<B, R>(fields: &[(&[u8], &[u8])]) -> R
where
    B: DeserializeRosBuilder<R> + Default,
    B::Context: Default,
    R: DeserializeRosResource,
{
    let mut builder = B::init(&B::Context::default());
    for (key, value) in fields {
        let result = builder.append_field(key, Some(value));
        assert!(
            matches!(result, AppendFieldResult::Appended),
            "field {} = {} was not accepted: {:?}",
            String::from_utf8_lossy(key),
            String::from_utf8_lossy(value),
            result
        );
    }
    builder.build().expect("all required fields were supplied")
}

/// Each target reports only its own fields: the remote ones stay empty on the memory action,
/// so a target without them matches it.
#[test]
fn decodes_the_actions_of_routeros() {
    let memory: SystemLoggingAction = build::<SystemLoggingActionBuilder, _>(&[
        (b".id", b"*0"),
        (b"name", b"memory"),
        (b"target", b"memory"),
        (b"memory-lines", b"1000"),
        (b"memory-stop-on-full", b"false"),
        (b"default", b"true"),
        (b"managed", b"false"),
    ]);
    assert_eq!(memory.cfg.remote, None);
    assert!(memory.status.default);
    let remote: SystemLoggingAction = build::<SystemLoggingActionBuilder, _>(&[
        (b".id", b"*3"),
        (b"name", b"remote"),
        (b"target", b"remote"),
        (b"remote", b"2a02:16a:fe00:1a0::10:8"),
        (b"remote-port", b"514"),
        (b"src-address", b"0.0.0.0"),
        (b"remote-log-format", b"cef"),
        (b"remote-protocol", b"udp"),
        (b"cef-event-delimiter", b""),
        (b"syslog-time-format", b"iso8601"),
        (b"vrf", b"main"),
        (b"default", b"true"),
        (b"managed", b"false"),
    ]);
    assert_eq!(remote.cfg.remote_port, Some(514));
    // an empty delimiter is reported as none: a target without one sends `=""`
    assert_eq!(remote.cfg.cef_event_delimiter, None);
}

#[test]
fn decodes_a_rule() {
    let rule: SystemLogging = build::<SystemLoggingBuilder, _>(&[
        (b".id", b"*5"),
        (b"topics", b"ipsec,!debug"),
        (b"prefix", b""),
        (b"regex", b""),
        (b"action", b"remote"),
        (b"disabled", b"false"),
        (b"default", b"false"),
        (b"invalid", b"false"),
        (b"managed", b"false"),
    ]);
    assert_eq!(rule.cfg.topics.len(), 2);
    assert_eq!(rule.cfg.prefix, None);
}
