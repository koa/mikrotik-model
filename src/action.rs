//! The catalog of actions: commands that are no `print`, `add`, `set`, `remove` or `move` of a
//! modeled menu (those are in `command`), each a type with its effect. Their replies are
//! [`SimpleResult`] rows; a monitor whose fields a menu's model has goes through
//! `command::MonitorOnce` instead.
use crate::{model::DeviceMode, value::RosValue};
use mikrotik_api::{
    command::{Observe, SessionBound, Write},
    prelude::{Command, CommandBuilder},
    simple::SimpleResult,
};
use std::{borrow::Cow, net::IpAddr, time::Duration};

/// Implements [`Command`] for an action: its effect, its path, the attributes it sends and,
/// optionally, its reply timeout.
macro_rules! action {
    ($name:ident $(<$lt:lifetime>)?, $effect:ty, $path:literal, |$this:ident, $builder:ident| $build:expr $(, timeout = |$t:ident| $timeout:expr)?) => {
        impl$(<$lt>)? Command for $name $(<$lt>)? {
            type Effect = $effect;
            type Reply = SimpleResult;
            fn path(&self) -> Cow<'_, [u8]> {
                Cow::Borrowed($path)
            }
            fn build(&self, $builder: CommandBuilder) -> CommandBuilder {
                let $this = self;
                $build
            }
            fn context(&self) {}
            $(
                fn reply_timeout(&self) -> Option<Duration> {
                    let $t = self;
                    $timeout
                }
            )?
        }
    };
}

/// `/execute`: runs `script` in the background on the device; the answer comes at once.
pub struct Execute<'a> {
    pub script: &'a str,
}
action!(Execute<'a>, Write, b"/execute", |this, builder| builder
    .attribute(b"script", this.script.as_bytes()));

/// `/ping` from the device.
pub struct Ping {
    pub address: IpAddr,
    pub count: u8,
    pub interval: Duration,
}
action!(Ping, Observe, b"/ping", |this, builder| builder
    .attribute(b"address", this.address.to_string().as_bytes())
    .attribute(b"count", this.count.to_string().as_bytes())
    .attribute(
        b"interval",
        format!("{}", this.interval.as_secs_f32()).as_bytes()
    ));

/// `/interface/ethernet/monitor` of the ports `numbers` (names, comma separated), once;
/// answered in their order.
pub struct EthernetMonitorOnce<'a> {
    pub numbers: &'a str,
}
action!(
    EthernetMonitorOnce<'a>,
    Observe,
    b"/interface/ethernet/monitor",
    |this, builder| builder
        .attribute(b"numbers", this.numbers.as_bytes())
        .attribute(b"once", b"")
);

/// `/interface/ethernet/poe/monitor` of the ports `numbers` (names, comma separated), once.
pub struct PoeMonitorOnce<'a> {
    pub numbers: &'a str,
}
action!(
    PoeMonitorOnce<'a>,
    Observe,
    b"/interface/ethernet/poe/monitor",
    |this, builder| builder
        .attribute(b"numbers", this.numbers.as_bytes())
        .attribute(b"once", b"")
);

/// `/interface/wireless/monitor` of the radios `numbers` (names, comma separated), once;
/// answered in their order.
pub struct WirelessMonitorOnce<'a> {
    pub numbers: &'a str,
}
action!(
    WirelessMonitorOnce<'a>,
    Observe,
    b"/interface/wireless/monitor",
    |this, builder| builder
        .attribute(b"numbers", this.numbers.as_bytes())
        .attribute(b"once", b"")
);

/// `/interface/ethernet/poe/power-cycle`: the port's PoE off for `duration`, then on again.
pub struct PowerCycle<'a> {
    pub port: &'a str,
    pub duration: Duration,
}
action!(
    PowerCycle<'a>,
    Write,
    b"/interface/ethernet/poe/power-cycle",
    |this, builder| builder
        .attribute(b"numbers", this.port.as_bytes())
        .attribute(
            b"duration",
            format!("{}s", this.duration.as_secs()).as_bytes()
        )
);

/// The radio menu whose CAPs' radios [`RadioProvision`] provisions again.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum RadioMenu {
    /// `/interface/wifi/radio`
    Wifi,
    /// `/caps-man/radio`
    CapsMan,
}

/// `provision` of the radios `numbers` (ids, comma separated) of `menu`.
pub struct RadioProvision<'a> {
    pub menu: RadioMenu,
    pub numbers: &'a str,
}
impl Command for RadioProvision<'_> {
    type Effect = Write;
    type Reply = SimpleResult;
    fn path(&self) -> Cow<'_, [u8]> {
        Cow::Borrowed(match self.menu {
            RadioMenu::Wifi => b"/interface/wifi/radio/provision",
            RadioMenu::CapsMan => b"/caps-man/radio/provision",
        })
    }
    fn build(&self, builder: CommandBuilder) -> CommandBuilder {
        builder.attribute(b"numbers", self.numbers.as_bytes())
    }
    fn context(&self) {}
}

/// `/partitions/activate`: boots `number` (its `.id`) at the next reboot.
pub struct PartitionActivate<'a> {
    pub number: &'a str,
}
action!(
    PartitionActivate<'a>,
    Write,
    b"/partitions/activate",
    |this, builder| builder.attribute(b"number", this.number.as_bytes())
);

/// `/partitions/copy-to`: the running RouterOS with its configuration to `target`. It
/// answers while it copies, `reply_timeout` apart at most.
pub struct PartitionCopyTo<'a> {
    pub target: &'a str,
    pub reply_timeout: Duration,
}
action!(
    PartitionCopyTo<'a>,
    Write,
    b"/partitions/copy-to",
    |this, builder| builder.attribute(b"number", this.target.as_bytes()),
    timeout = |this| Some(this.reply_timeout)
);

/// `/partitions/save-config-to`: the configuration alone to `target`.
pub struct PartitionSaveConfigTo<'a> {
    pub target: &'a str,
    pub reply_timeout: Duration,
}
action!(
    PartitionSaveConfigTo<'a>,
    Write,
    b"/partitions/save-config-to",
    |this, builder| builder.attribute(b"number", this.target.as_bytes()),
    timeout = |this| Some(this.reply_timeout)
);

/// `/partitions/repartition`: splits the flash into `partitions`, reformats and reboots.
pub struct Repartition {
    pub partitions: u8,
}
action!(
    Repartition,
    Write,
    b"/partitions/repartition",
    |this, builder| builder.attribute(b"partitions", this.partitions.to_string().as_bytes())
);

/// `/system/reboot`.
pub struct Reboot;
action!(Reboot, Write, b"/system/reboot", |_this, builder| builder);

/// `/system/shutdown`.
pub struct Shutdown;
action!(Shutdown, Write, b"/system/shutdown", |_this, builder| {
    builder
});

/// `/system/backup/save` of the configuration as `name.backup`, encrypted with `password`.
pub struct BackupSave<'a> {
    pub name: &'a str,
    pub password: &'a str,
}
action!(
    BackupSave<'a>,
    Write,
    b"/system/backup/save",
    |this, builder| builder
        .attribute(b"name", this.name.as_bytes())
        .attribute(b"password", this.password.as_bytes())
);

/// `/system/device-mode/update`: turns `features` on, and changes to `mode` where given, to be
/// confirmed at the device within `activation_timeout`. It counts down over its connection,
/// which cancels it when it closes, so it waits as long as the connection lasts.
pub struct DeviceModeUpdate<'a> {
    pub mode: Option<&'a DeviceMode>,
    pub features: &'a [&'a str],
    pub activation_timeout: Duration,
}
impl Command for DeviceModeUpdate<'_> {
    type Effect = SessionBound;
    type Reply = SimpleResult;
    fn path(&self) -> Cow<'_, [u8]> {
        Cow::Borrowed(b"/system/device-mode/update")
    }
    fn build(&self, builder: CommandBuilder) -> CommandBuilder {
        let builder = match self.mode {
            Some(mode) => builder.attribute(b"mode", mode.encode_ros().as_ref()),
            None => builder,
        };
        self.features
            .iter()
            .fold(builder, |builder, feature| {
                builder.attribute(feature.as_bytes(), b"yes")
            })
            .attribute(
                b"activation-timeout",
                format!("{}s", self.activation_timeout.as_secs()).as_bytes(),
            )
    }
    fn context(&self) {}
    fn reply_timeout(&self) -> Option<Duration> {
        None
    }
}

/// `/system/package/uninstall` of the package `number` (its `.id`), at the next reboot.
pub struct PackageUninstall<'a> {
    pub number: &'a str,
}
action!(
    PackageUninstall<'a>,
    Write,
    b"/system/package/uninstall",
    |this, builder| builder.attribute(b"numbers", this.number.as_bytes())
);

/// `/system/package/downgrade`: installs the uploaded older packages and reboots.
pub struct PackageDowngrade;
action!(
    PackageDowngrade,
    Write,
    b"/system/package/downgrade",
    |_this, builder| builder
);

/// `/system/routerboard/upgrade`: the firmware loads at the next boot.
pub struct RouterboardUpgrade;
action!(
    RouterboardUpgrade,
    Write,
    b"/system/routerboard/upgrade",
    |_this, builder| builder
);

/// `/certificate/import` of the file `file_name` as `name`; RouterOS takes the file away.
pub struct CertificateImport<'a> {
    pub file_name: &'a str,
    pub passphrase: &'a str,
    pub name: &'a str,
}
action!(
    CertificateImport<'a>,
    Write,
    b"/certificate/import",
    |this, builder| builder
        .attribute(b"file-name", this.file_name.as_bytes())
        .attribute(b"passphrase", this.passphrase.as_bytes())
        .attribute(b"name", this.name.as_bytes())
);

/// `/user/set password`: a user's password, which no `print` shows.
pub struct UserPassword<'a> {
    pub user: &'a str,
    pub password: &'a str,
}
action!(UserPassword<'a>, Write, b"/user/set", |this, builder| {
    builder
        .attribute(b"numbers", this.user.as_bytes())
        .attribute(b"password", this.password.as_bytes())
});

/// `/system/script/job/remove` of the job `number` (its `.id`): stops a running script. An API
/// session is a job too; never remove those wholesale.
pub struct ScriptJobRemove<'a> {
    pub number: &'a str,
}
action!(
    ScriptJobRemove<'a>,
    Write,
    b"/system/script/job/remove",
    |this, builder| builder.attribute(b"numbers", this.number.as_bytes())
);
