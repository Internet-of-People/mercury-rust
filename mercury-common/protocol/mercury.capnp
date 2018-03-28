@0xbf11c96f54b8924d;



using PublicKey = Data;
using ProfileId = Data;
using Signature = Data;
using ApplicationId = Text;
using AppMessageFrame = Data;



struct Profile
{
    # data      @0 : Data; # output of multicodec encode()

    # NOTE these are mandatory in the API, but will be serialized into the data instead
    id        @0 : ProfileId;
    publicKey @1 : PublicKey;
}


interface ProfileRepo
{
    # TODO what filter criteria should we have in list()?
    list @0 () -> (profiles: List(Profile));
    load @1 (profileId: ProfileId) -> (profile: Profile);
    resolve @2 (profileUrl: Text) -> (profile: Profile);
}



struct RelationHalfProof
{
    data            @0 : Data;
    # relationType    @0 : Text,
    # myId            @1 : ProfileId,
    # mySign          @2 : Signature,
    # peerId          @3 : ProfileId,
}


struct RelationProof
{
    data        @0 : Data;
    # halfProof   @0 : RelationHalfProof,
    # peerSign    @1 : Signature,
}


struct HomeInvitation
{
    data @0 : Data;
}


struct OwnProfile
{
    profile     @0 : Profile;
    privateData @1 : Data; # TODO maybe multicodec output?
}



interface AppMessageListener
{
    onMessageSent @0 (message: AppMessageFrame);
}



interface Home extends (ProfileRepo)
{
    claim @0 (profileId: ProfileId) -> (ownProfile: OwnProfile);
    register @1 (ownProfile: OwnProfile, invite: HomeInvitation) -> (ownProfile: OwnProfile);
    login @2 (profileId : ProfileId) -> (session : HomeSession);

    pairRequest @3 (halfProof: RelationHalfProof);  # NOTE called on acceptor's home
    pairResponse @4 (relationProof: RelationProof);      # NOTE called on requestor's home

    #call @5 (relation: RelationProof, app: ApplicationId, initPayload: AppMessageFrame,
    #         toCaller: AppMessageListener) -> (toCallee: CallMessages);
}



struct Call
{
    callerId    @0 : ProfileId;
    initPayload @1 : AppMessageFrame;
    toCaller    @2 : AppMessageListener;
}

interface CallListener
{
    onCall @0 (call: Call) -> (toCallee: AppMessageListener);
}



interface HomeEventNotifier
{
    # TODO
}


interface HomeSession
{
    update @0 (ownProfile: OwnProfile);
    unregister @1 (newHome: ProfileId); # NOTE closes session after successful call

    # events @2 () -> (events: HomeEventNotifier);
    # checkinApp @3 (app: ApplicationId) -> (calls: Calls);

    # TODO remove after testing
    ping @2 (txt : Text) -> (pong : Text);
}
