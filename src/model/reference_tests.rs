//! The references an entity consumes, which order the mutations
//! (`sort_mutations_with_provided_dependencies`).

use crate::ascii::AsciiString;
use crate::model::{InterfaceWifiProvisioningCfg, ReferenceType};
use crate::resource::RosResource;

/// An optional list (`o;mu`, `slave-configurations` of `/interface/wifi/provisioning`)
/// refers to each of its elements, not to the list as one name: a provisioning with two
/// slaves otherwise waits for a configuration named `a,b`, which never comes.
#[test]
fn optional_list_refers_to_each_element() {
    let provisioning = InterfaceWifiProvisioningCfg {
        master_configuration: Some(AsciiString::from("master")),
        slave_configurations: Some(["a", "b"].into_iter().map(AsciiString::from).collect()),
        ..Default::default()
    };
    let mut configurations = provisioning
        .consumes_reference()
        .filter(|(reference, _)| *reference == ReferenceType::WifiConfiguration)
        .map(|(_, name)| String::from_utf8_lossy(&name).into_owned())
        .collect::<Vec<_>>();
    configurations.sort();
    assert_eq!(configurations, ["a", "b", "master"]);
}
