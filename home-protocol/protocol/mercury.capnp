@0xbf11c96f54b8924d;


### TODO returning errors from the server and differentiating
#        remote errors from internal capnp errors is not solved/implemented yet


# NOTE that though these types all hold complex serialized data,
#      communication must not depend on their format and interpretation
using PublicKey = Data;
using ProfileId = Data;
using Signature = Data;
using ApplicationId = Text;
using AppMessageFrame = Data;
using HomeInvitation = Data;
using Profile = Data;



interface ProfileRepo
{
    get @0 (profileId: ProfileId) -> (profile: Profile);

    # TODO what filter criteria should we have in list()?
    # list @1 () -> (profiles: List(Profile));
    # resolve @2 (profileUrl: Text) -> (profile: Profile);
}

struct RelationHalfProof
{
    relationType    @0 : Text;
    signerId        @1 : ProfileId;
    peerId          @2 : ProfileId;
    signature       @3 : Signature;
}

struct RelationProof
{
    relationType    @0 : Text;
    aId             @1 : ProfileId;
    aSignature      @2 : Signature;
    bId             @3 : ProfileId;
    bSignature      @4 : Signature;
}

struct OwnProfile
{
    profile     @0 : Profile;
    privateData @1 : Data;
}



interface AppMessageListener
{
    receive @0 (message: AppMessageFrame);
    error   @1 (error: Text);
}



interface Home extends (ProfileRepo)
{
    claim @0 (profileId: ProfileId) -> (ownProfile: OwnProfile);
    register @1 (ownProfile: OwnProfile, halfProof: RelationHalfProof, invite: HomeInvitation) -> (ownProfile: OwnProfile);
    login @2 (proofOfHome : RelationProof) -> (session : HomeSession);

    pairRequest @3 (halfProof: RelationHalfProof);  # NOTE called on acceptor's home
    pairResponse @4 (relation: RelationProof); # NOTE called on requestor's home

    call @5 (relation: RelationProof, app: ApplicationId, initPayload: AppMessageFrame,
             toCaller: AppMessageListener) -> (toCallee: AppMessageListener);
}



struct CallRequest
{
    relation    @0 : RelationProof;
    initPayload @1 : AppMessageFrame;
    toCaller    @2 : AppMessageListener;
}

interface CallListener
{
    receive @0 (call: CallRequest) -> (toCallee: AppMessageListener);
    error   @1 (error: Text);
}



struct ProfileEvent
{
    union
    {
        # TODO maybe we could optimize pairing data by omitting most fields, signature and sender profile_id is mandatory
        unknown         @0 : Data;
        pairingRequest  @1 : RelationHalfProof;
        pairingResponse @2 : RelationProof;
    }
}

interface ProfileEventListener
{
    receive @0 (event: ProfileEvent);
    error   @1 (error: Text);
}


interface HomeSession
{
    update @0 (ownProfile: OwnProfile);
    unregister @1 (newHome: Profile); # NOTE closes session after successful call

    events @2 (eventListener: ProfileEventListener);
    checkinApp @3 (app: ApplicationId, callListener: CallListener);

    # TODO consider removing this after testing
    ping @4 (txt : Text) -> (pong : Text);
}
