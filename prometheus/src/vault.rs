use failure::Fallible;

use morpheus_storage::*;

pub trait ProfileVault {
    fn list(&self) -> Fallible<Vec<ProfileId>>;
    fn create_id(&self) -> Fallible<ProfileId>;

    fn get_active(&self) -> Fallible<Option<ProfileId>>;
    fn set_active(&self, id: &ProfileId) -> Fallible<()>;
}

// TODO remove this dummy implementation completely and use the RpcProfileStore instead
pub struct DummyProfileVault {
    pub profile_id: ProfileId,
}

impl DummyProfileVault {
    pub fn new() -> Self {
        let profile_id = "Iez21JXEtMzXjbCK6BAYFU9ewX".parse::<ProfileId>().unwrap();
        Self { profile_id }
    }
}

impl ProfileVault for DummyProfileVault {
    fn list(&self) -> Fallible<Vec<ProfileId>> {
        let active_opt = self.get_active()?;
        Ok(vec![active_opt.unwrap()])
    }

    fn create_id(&self) -> Fallible<ProfileId> {
        Ok(self.get_active()?.unwrap())
    }

    fn get_active(&self) -> Fallible<Option<ProfileId>> {
        Ok(Some(self.profile_id.clone()))
    }
    fn set_active(&self, id: &ProfileId) -> Fallible<()> {
        unimplemented!()
    }
}
