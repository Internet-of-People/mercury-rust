@0xbf11c96f54b8924d;


using PublicKey = Data;
using ProfileId = Data;
using Signature = Data;
using ApplicationId = Text;
#using AppMessageFrame = Data;


struct Profile
{
    id        @0 : ProfileId;
    publicKey @1 : PublicKey;
# TODO pub facets:     Vec<ProfileFacet>;
}


struct Contact
{
    # TODO
}


struct HomeInvitation
{
    # TODO
}


struct OwnProfileData
{
    profile     @0 : Profile;
    privateData @1 : Data; # TODO maybe multicodec output?
}


interface Signer
{
    # TODO
}


struct OwnProfile
{
    data   @0 : OwnProfileData;
    signer @1 : Signer;
}



interface ProfileRepo
{
    # TODO what filter criteria should we have here?
#    list @0 () -> (profiles: List(Profile));
#    load @1 (profileId: ProfileId) -> (profile: Profile);
#    resolve @2 (profileUrl: Text) -> (profile: Profile);
}


# struct Call {}
# interface ClientNotificator {
#     incoming_call @0 (call: Call);
# }

struct CallMessages
{
    # TODO
}


interface Home extends (ProfileRepo)
{
#    register @0 (ownProfile: OwnProfile, invite: HomeInvitation) -> (ownProfile: OwnProfile);
#    update @1 (ownProfile: OwnProfile) -> (ownProfile: OwnProfile);
#    unregister @2 (ownProfile: OwnProfile, newHome: Profile) -> (ownProfile: OwnProfile);

    # TODO consider changing argument to profile: ProfileId so you don't have to
    #      download the public data, or you already have the id stored
#    claim @3 (profile: Profile, signer: Signer) -> (ownProfile: OwnProfile);

    # NOTE acceptor must have this server as its home
#    pairWith @4 (initiator: OwnProfile, acceptor: Profile) -> (contact: Contact);
#    call @5 (caller: OwnProfile, callee: Contact, appId: ApplicationId, initPayload: Data) ->
#        (messages: CallMessages);

#    login @6 (profile : OwnProfileData) -> (session : HomeSession);
    login @0 (name : Text) -> (session : HomeSession);
#    fn login(&self, own_prof: OwnProfile) ->
#        Box< Future<Item=Box<HomeSession>, Error=ErrorToBeSpecified> >;
}


interface HomeEventNotifier
{
    # TODO
}

interface Calls
{
    # TODO
}

interface HomeSession
{
#    events @0 () -> (events: HomeEventNotifier);
#    checkinApp @1 (app: ApplicationId) -> (calls: Calls);

    # TODO remove after testing
    ping @0 (txt : Text) -> (pong : Text);
}
