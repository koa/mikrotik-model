use crate::MikrotikDevice;
use crate::resource::Error;

pub trait CanAddRepository {
    type Item;
    fn add(&mut self, item: Self::Item);
}

pub trait SingleItemRepository {
    type ReadOnlyItem;
    type ReadWriteItem;
    fn get(&self) -> (&Self::ReadOnlyItem, &Self::ReadWriteItem);
}

pub trait CanUpdateRepository {
    type Item;
    type Key;
    fn replace(&mut self, key: Self::Key, item: Self::Item);
    fn get_mut(&mut self, key: &Self::Key) -> Option<&mut Self::Item>;
}

pub trait Repository: Sized {
    async fn fetch(device: &MikrotikDevice) -> Result<Self, Error>;
}

mod test_repos {
    use super::*;
    use crate::model::SystemRouterboardSettingsCfg;
    use crate::resource::SingleResource;
    struct RouterboardSettingsRepository {
        data: SystemRouterboardSettingsCfg,
    }

    impl Repository for RouterboardSettingsRepository {
        async fn fetch(device: &MikrotikDevice) -> Result<Self, Error> {
            Ok(RouterboardSettingsRepository {
                data: SystemRouterboardSettingsCfg::fetch(device)
                    .await?
                    .ok_or(Error::ErrorFetchingSingleItem)?,
            })
        }
    }
}
