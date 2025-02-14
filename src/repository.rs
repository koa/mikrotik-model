use crate::resource::Error;
use crate::MikrotikDevice;

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
    use crate::model::SystemResource;
    use crate::resource::SingleResource;
    struct SystemResourcesRepository {
        data: SystemResource,
    }

    impl Repository for SystemResourcesRepository {
        async fn fetch(device: &MikrotikDevice) -> Result<Self, Error> {
            Ok(SystemResourcesRepository {
                data: SystemResource::fetch(device)
                    .await?
                    .ok_or(Error::ErrorFetchingSingleItem)?,
            })
        }
    }
}
