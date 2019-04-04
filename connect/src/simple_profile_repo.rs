extern crate futures;

use std::cell::RefCell;
//use std::collections::HashMap;
use std::rc::Rc;

use failure::Fail;
use futures::prelude::*;

use mercury_home_protocol::{*, error::*};
use mercury_storage::asynch::{KeyValueStore, imp::InMemoryStore};



pub struct SimpleProfileRepo {
    profiles : Rc<RefCell< KeyValueStore<ProfileId, Profile> >>
}

impl Default for SimpleProfileRepo {
    fn default() -> Self { InMemoryStore::new().into() }
}

impl<T: KeyValueStore<ProfileId,Profile> + 'static> From<T> for SimpleProfileRepo {
    fn from(src: T) -> Self{ Self{ profiles: Rc::new( RefCell::new(src) ) } }
}


impl SimpleProfileRepo {
    pub fn insert(&self, profile: Profile)
        -> AsyncResult<(), ::mercury_storage::error::StorageError>
    {
        self.profiles.borrow_mut().set( profile.id.clone(), profile.clone() )
    }
}


impl ProfileRepo for SimpleProfileRepo
{
//    /// List all profiles that can be load()'ed or resolve()'d.
//    fn list(&self, /* TODO what filter criteria should we have here? */ ) ->
//        HomeStream<Profile,String>
//    {
//        unimplemented!()
//    }

    /// Look for specified `id` and return. This might involve searching for the latest version
    /// of the profile in the dht, but if it's the profile's home server, could come from memory, too.
    fn load(&self, id: &ProfileId) -> AsyncResult<Profile, Error>
    {
        let fut = self.profiles.borrow().get( id.to_owned() )
            .map_err( |e| e.context(ErrorKind::ProfileLookupFailed).into() );
        Box::new(fut)
    }


//    fn resolve(&self, _url: &str) -> AsyncResult<Profile, Error>
//    {
//        unimplemented!()
//    }
}