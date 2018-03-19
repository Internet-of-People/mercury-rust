@0xbf11c96f54b8924d;


struct Profile
{
    id        @0 : Data;
    publicKey @1 : Data;
# TODO pub facets:     Vec<ProfileFacet>;
}


struct OwnProfileData
{
    profile     @0 : Profile;
    privateData @1 : Data; # TODO maybe multicodec output?
}



interface Home
{
    login @0 (name : Text) -> (session : HomeSession);
#     login @0 (profile : OwnProfileData) -> (session : HomeSession);
}



# struct Call {}
# interface ClientNotificator {
#     incoming_call @0 (call: Call);
# }

interface HomeSession
{
    ping @0 (txt : Text) -> (pong : Text);
#    checkin_app @0 (app: Text) -> (result : Client);
}
