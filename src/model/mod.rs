/*use crate::{
resource, value::{self, IpOrInterface},
ascii,
};
use std::{time::Duration, net::IpAddr};
use mac_address::MacAddress;
use ipnet::IpNet;*/
use value::Id;

include!(concat!(env!("OUT_DIR"), "/mikrotik-model.rs"));

mod defaults;
mod enums;

/*impl resource::Updatable<InterfaceBridgePortById> for InterfaceBridgePortCfg {
    fn calculate_update<'a>(
        &'a self,
        from: &'a InterfaceBridgePortById,
    ) -> resource::ResourceMutation<'a> {
        resource::ResourceMutation {
            resource: <Self as resource::RosResource>::path(),
            operation: resource::ResourceMutationOperation::UpdateByKey(value::KeyValuePair {
                key: <InterfaceBridgePortById as resource::KeyedResource>::key_name(),
                value: value::RosValue::encode_ros(&from.id),
            }),
            fields: resource::SetResource::changed_values(self, from).collect(),
            depends: Box::new([]),
            provides: Box::new([]),
        }
    }
}
*/
