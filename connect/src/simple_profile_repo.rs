extern crate futures;

use std::collections::HashMap;
use mercury_home_protocol::*;
use futures::{Future, future};


pub struct SimpleProfileRepo {
    profiles : HashMap<ProfileId, Profile>
}

impl SimpleProfileRepo {
    pub fn new() -> SimpleProfileRepo {
        SimpleProfileRepo { profiles: HashMap::new() }
    }

    pub fn insert(&mut self, profile: Profile) -> Option<Profile> {
        self.profiles.insert(profile.id.clone(), profile.clone())
    }

}

impl ProfileRepo for SimpleProfileRepo {
    /// List all profiles that can be load()'ed or resolve()'d.
    fn list(&self, /* TODO what filter criteria should we have here? */ ) ->
        HomeStream<Profile,String>
    {
        unimplemented!()
//        let (send, recv) = futures::sync::mpsc::channel(0);
//        recv
    }

    /// Look for specified `id` and return. This might involve searching for the latest version
    /// of the profile in the dht, but if it's the profile's home server, could come from memory, too.
    fn load(&self, id: &ProfileId) ->
        Box< Future<Item=Profile, Error=Error> >
    {
        match self.profiles.get(id) {
            Some(profile) => Box::new(future::ok(profile.to_owned())),
            None => Box::new(future::err(ErrorKind::ProfileLookupFailed.into()))
        }
    }


    /// Same as load(), but also contains hints for resolution, therefore it's more efficient than load(id)
    ///
    /// The `url` may contain
    /// * ProfileID (mandatory)
    /// * some profile metadata (for user experience enhancement) (big fat warning should be thrown if it does not match the latest info)
    /// * ProfileID of its home server
    /// * last known multiaddress(es) of its home server
    fn resolve(&self, url: &str) ->
        Box< Future<Item=Profile, Error=Error> >
    {
        self.load(&ProfileId(Vec::from(url)))
    }

}