use std::rc::Rc;

use futures::prelude::*;

use mercury_home_protocol::*;
use mercury_storage::async::KeyValueStore;
use ::sdk::DAppApi;



#[derive(Clone, Debug, Deserialize, Eq, Hash, PartialEq, PartialOrd, Serialize)]
pub struct DAppAction(Vec<u8>);

#[derive(Clone, Debug, Deserialize, Eq, Hash, PartialEq, PartialOrd, Serialize)]
pub struct DeviceAuthorization(Vec<u8>);

#[derive(Clone, Debug, Deserialize, Eq, Hash, PartialEq, PartialOrd, Serialize)]
pub struct DAppPermission(Vec<u8>);

#[derive(Clone, Debug, Deserialize, Eq, Hash, PartialEq, PartialOrd, Serialize)]
pub struct Bip32Path(String);



// Hierarchical deterministic seed for identity handling to generate profiles
pub trait KeyVault
{
    // Get the next hierarchical path to generate a new profile with
    fn next(&self) -> Bip32Path;

    // TODO what do we need here to unlock the private key? Maybe a password?
    // Get or create an empty profile for a path returned by next()
    fn unlock_profile(&self, bip32_path: &Bip32Path) -> Rc<Signer>;
}


// Usage of Bip32 hierarchy, format: path => data stored with that key
pub trait Bip32PathMapper
{
    // master_seed/purpose_mercury => last_profile_number and profile {id: number} map
    fn root_path(&self) -> Bip32Path;

    // m/mercury/profile_number => list of relations, apps, etc
    fn profile_path(&self, profile_id: &ProfileId) -> Bip32Path;

    // m/mercury/profile/app_id => application-specific data
    fn app_path(&self, profile_id: &ProfileId, app_id: &ApplicationId) -> Bip32Path;
}


pub trait AccessManager
{
    fn ask_read_access(&self, resource: &Bip32Path) ->
        Box< Future<Item=PublicKey, Error=ErrorToBeSpecified> >;

    fn ask_write_access(&self, resource: &Bip32Path) ->
        Box< Future<Item=Rc<Signer>, Error=ErrorToBeSpecified> >;
}



// User interface (probably implemented with platform-native GUI) for actions
// that are initiated by the SDK and require some kind of user interaction
pub trait UserInterface
{
    // Initialize system components and configuration where user interaction is needed,
    // e.g. HD wallets need manually saving generated new seed or entering old one
    fn initialize(&self) -> Box< Future<Item=(), Error=ErrorToBeSpecified> >;

    // An action requested by a distributed application needs
    // explicit user confirmation.
    // TODO how to show a human-readable summary of the action (i.e. binary to be signed)
    //      making sure it's not a fake/misinterpreted description?
    fn confirm(&self, action: &DAppAction)
        -> Box< Future<Item=Signature, Error=ErrorToBeSpecified> >;

    // Select a profile to be used by a dApp. It can be either an existing one
    // or the user can create a new one (using a KeyVault) to be selected.
    // TODO this should open something nearly identical to manage_profiles()
    fn select_profile(&self)
        -> Box< Future<Item=Profile, Error=ErrorToBeSpecified> >;

    // Open profiles with new, delete and edit (e.g. homes, contacts, apps, etc) options.
    // Specific profiles can also be set online/offline.
    // TODO it could look something like:
    //      Profiles
    //      [x]ON  business (edit) (delete)
    //      [ ]off family   (edit) (delete)
    //      [x]ON  hobby    (edit) (delete)
    //      (new profile)
    fn manage_profiles(&self)
        -> Box< Future<Item=(), Error=ErrorToBeSpecified> >;
}



pub trait ConnectService
{
    // TODO some kind of proof might be needed that the AppId given really belongs to the caller
    // NOTE this implicitly asks for user interaction (through UI) selecting a profile to be used with the app
    fn app_api(&self, app: &ApplicationId)
        -> Box< Future<Item=Rc<DAppApi>, Error=ErrorToBeSpecified> >;

    // TODO The Settings app is not really a dApp but a privilegized system app, might use different authorization
    fn settings(&self, authorization: &DAppPermission)
        -> Box< Future<Item=Rc<ServiceSettings>, Error=ErrorToBeSpecified> >;
}


pub trait ServiceSettings
{
    fn profiles(&self)
        -> Box< Future<Item=Vec<Profile>, Error=ErrorToBeSpecified> >;

    fn create_profile(&self)
        -> Box< Future<Item=Vec<OwnProfile>, Error=ErrorToBeSpecified> >;

    fn update_profile(&self, profile: &OwnProfile)
        -> Box< Future<Item=(), Error=ErrorToBeSpecified> >;

    fn remove_profile(&self, profile: &ProfileId)
        -> Box< Future<Item=(), Error=ErrorToBeSpecified> >;


    fn homes(&self, profile: &ProfileId)
        -> Box< Future<Item=Vec<ProfileId>, Error=ErrorToBeSpecified> >;

    fn join_home(&self, profile: &ProfileId, home: &ProfileId)
        -> Box< Future<Item=(), Error=ErrorToBeSpecified> >;

    fn leave_home(&self, profile: &ProfileId, home: &ProfileId)
        -> Box< Future<Item=(), Error=ErrorToBeSpecified> >;


    fn relations(&self) ->
        Box< Future<Item=Vec<::Relation>, Error=ErrorToBeSpecified> >;

    fn initiate_relation(&self, with_profile: &ProfileId) ->
        Box< Future<Item=(), Error=ErrorToBeSpecified> >;

    fn accept_relation(&self, half_proof: &RelationHalfProof) ->
        Box< Future<Item=(), Error=ErrorToBeSpecified> >;

    fn revoke_relation(&self, relation: &RelationProof) ->
        Box< Future<Item=(), Error=ErrorToBeSpecified> >;
}



pub struct ServiceImpl
{
    keyvault:   Rc<KeyVault>,
    pathmap:    Rc<Bip32PathMapper>,
    accessman:  Rc<AccessManager>,
    ui:         Rc<UserInterface>,
    profiles:   Rc<KeyValueStore<ProfileId, OwnProfile>>,
}


impl ServiceSettings for ServiceImpl
{
    fn profiles(&self)
        -> Box< Future<Item=Vec<Profile>, Error=ErrorToBeSpecified> >
    {
        unimplemented!()
    }

    fn create_profile(&self)
        -> Box< Future<Item=Vec<OwnProfile>, Error=ErrorToBeSpecified> >
    {
        unimplemented!()
    }

    fn update_profile(&self, profile: &OwnProfile)
        -> Box< Future<Item=(), Error=ErrorToBeSpecified> >
    {
        unimplemented!()
    }

    fn remove_profile(&self, profile: &ProfileId)
        -> Box< Future<Item=(), Error=ErrorToBeSpecified> >
    {
        unimplemented!()
    }


    fn homes(&self, profile: &ProfileId)
        -> Box< Future<Item=Vec<ProfileId>, Error=ErrorToBeSpecified> >
    {
        unimplemented!()
    }

    fn join_home(&self, profile: &ProfileId, home: &ProfileId)
        -> Box< Future<Item=(), Error=ErrorToBeSpecified> >
    {
        unimplemented!()
    }

    fn leave_home(&self, profile: &ProfileId, home: &ProfileId)
        -> Box< Future<Item=(), Error=ErrorToBeSpecified> >
    {
        unimplemented!()
    }


    fn relations(&self) ->
        Box< Future<Item=Vec<::Relation>, Error=ErrorToBeSpecified> >
    {
        unimplemented!()
    }

    fn initiate_relation(&self, with_profile: &ProfileId) ->
        Box< Future<Item=(), Error=ErrorToBeSpecified> >
    {
        unimplemented!()
    }

    fn accept_relation(&self, half_proof: &RelationHalfProof) ->
        Box< Future<Item=(), Error=ErrorToBeSpecified> >
    {
        unimplemented!()
    }

    fn revoke_relation(&self, relation: &RelationProof) ->
        Box< Future<Item=(), Error=ErrorToBeSpecified> >
    {
        unimplemented!()
    }
}



//pub trait UiProfile
//{
//    fn homes(&self) -> Vec<RelationProof>;
//    fn profile(&self) -> OwnProfile;
//
//    fn update_profile(&self, own_profile: &OwnProfile) ->
//        Box< Future<Item=(), Error=ErrorToBeSpecified> >;
//}
