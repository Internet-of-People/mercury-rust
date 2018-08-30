use std::rc::Rc;

use futures::{Future, IntoFuture};
use tokio_core::reactor;

use mercury_storage::async::KeyValueStore;

use sdk_impl::DAppConnect;
use ::*;


pub struct Call
{
    pub sender   : AppMsgSink,
    pub receiver : AppMsgStream
}

pub trait DAppInit
{
    // Implies asking the user interface to manually pick a profile the app is used with
    fn initialize(&self, app: &ApplicationId, handle: &reactor::Handle)
        -> Box< Future<Item=Rc<DAppApi>, Error=Error> >;
}

pub trait DAppApi
{
    // Once initialized, the profile is selected and can be queried any time
    fn selected_profile(&self) -> &ProfileId;

    fn contacts(&self) -> Box< Future<Item=Vec<Relation>, Error=::Error> >;

    fn app_storage(&self) -> Box< Future<Item=KeyValueStore<String,String>, Error=::Error> >;

    fn checkin(&self) -> Box< Future<Item=HomeStream<Box<IncomingCall>,String>, Error=::Error> >;

    // This includes initiating a pair request with the profile if not a relation yet
    fn call(&self, profile_id: &ProfileId, init_payload: AppMessageFrame)
        -> Box< Future<Item=Call, Error=::Error> >;
}



impl DAppInit for Rc<ProfileGateway>
{
    fn initialize(&self, app: &ApplicationId, handle: &reactor::Handle)
        -> Box< Future<Item=Rc<DAppApi>, Error=Error> >
    {
        let instance = Rc::new( DAppConnect::new( self.clone(), app, handle) ) as Rc<DAppApi>;
        Box::new( Ok(instance).into_future() )
    }
}



//pub trait SdkProfileRepository
//{
//    fn create(&self, profile_path: Option<Bip32Path>, own_profile: &OwnProfile) ->
//        Box< Future<Item=(Rc<SdkProfile>, DeviceAuthorization), Error=ErrorToBeSpecified> >;
//
//    fn claim(&self, profile_path: Option<Bip32Path>, auth: Option<DeviceAuthorization>) ->
//        Box< Future<Item=Rc<SdkProfile>, Error=ErrorToBeSpecified> >;
//}
//
//
//
//pub trait SdkProfile
//{
//    fn relations(&self) ->
//        Box< Future<Item=Vec<Relation>, Error=ErrorToBeSpecified> >;
//
//    fn initiate_relation(&self, with_profile: &ProfileId) ->
//        Box< Future<Item=(), Error=ErrorToBeSpecified> >;
//
//    fn accept_relation(&self, half_proof: &RelationHalfProof) ->
//        Box< Future<Item=(), Error=ErrorToBeSpecified> >;
//
//    fn revoke_relation(&self, relation: &RelationProof) ->
//        Box< Future<Item=(), Error=ErrorToBeSpecified> >;
//
//
//    fn login(&self) ->
//        Vec<Box< Future<Item=Rc<HomeSession>, Error=ErrorToBeSpecified> >>;
//}
