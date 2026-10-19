//! Static decode tests for `/ip/socks` (`ros_model/ip/Socks.txt`), with the API output of a
//! factory CCR2004 (7.20.8) and of a user added on CHR 7.24.5, which reports the password.

use crate::model::IpSocksUsersCfgBuilder;
use crate::model::{IpSocksAuthMethod, IpSocksCfg, IpSocksCfgBuilder, IpSocksUsersCfg};
use crate::resource::{DeserializeRosBuilder, DeserializeRosResource};

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
            matches!(result, crate::resource::AppendFieldResult::Appended),
            "field {} = {} was not accepted: {:?}",
            String::from_utf8_lossy(key),
            String::from_utf8_lossy(value),
            result
        );
    }
    builder.build().expect("all required fields were supplied")
}

#[test]
fn factory_settings_are_the_default() {
    let socks: IpSocksCfg = build::<IpSocksCfgBuilder, _>(&[
        (b"enabled", b"false"),
        (b"port", b"1080"),
        (b"connection-idle-timeout", b"2m"),
        (b"max-connections", b"200"),
        (b"vrf", b"main"),
        (b"version", b"4"),
        (b"auth-method", b"none"),
    ]);
    assert_eq!(socks, IpSocksCfg::default());
    assert_eq!(socks.auth_method, IpSocksAuthMethod::None);
}

#[test]
fn decodes_a_user() {
    let user: IpSocksUsersCfg = build::<IpSocksUsersCfgBuilder, _>(&[
        (b"name", b"prov"),
        (b"password", b"geheim"),
        (b"only-one", b"false"),
        (b"disabled", b"false"),
    ]);
    assert_eq!(user.password.0.as_ref(), b"geheim");
}
