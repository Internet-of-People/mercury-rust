use super::*;
use mercury_connect::sdk::*;
use mercury_home_protocol::*;
use futures::IntoFuture;
use futures::Stream;
use std::rc::Rc;

pub struct Client {
    appcx : AppContext,
    cfg: ClientConfig,
    mercury_app: Rc<DAppApi>
}

impl Client{
    pub fn new(cfg: ClientConfig, appcx: AppContext, reactor: &mut Core) -> Self{
        //TODO privatekey things
        let privk = PrivateKey( b"\x83\x3F\xE6\x24\x09\x23\x7B\x9D\x62\xEC\x77\x58\x75\x20\x91\x1E\x9A\x75\x9C\xEC\x1D\x19\x75\x5B\x7D\xA9\x01\xB9\x6D\xCA\x3D\x42".to_vec() );
        let client_signer = Rc::new( Ed25519Signer::new(&privk).unwrap() );
        let mut profile_store = SimpleProfileRepo::new();
        let home_connector = SimpleTcpHomeConnector::new(reactor.handle());
        let profile_gw = Rc::new(ProfileGatewayImpl::new(client_signer, Rc::new(profile_store),  Rc::new(home_connector)));
        let dapi = reactor.run((profile_gw as Rc<ProfileGateway>).initialize(&ApplicationId("buttondapp".into()))).unwrap();
        Self{
            appcx: appcx,
            cfg: cfg,
            mercury_app: dapi,
        }
    }
}

impl IntoFuture for Client {
    type Item = ();
    type Error = std::io::Error;
    type Future = Box<Future<Item=Self::Item, Error=Self::Error>>;

    fn into_future(self) -> Self::Future {
        let callee_profile_id = self.cfg.callee_profile_id.clone();

        info!("application initialized, calling {:?}", callee_profile_id);
        let f = self.mercury_app.call(&callee_profile_id, AppMessageFrame(vec![]))
//            self.mercury_app.initialize(&ApplicationId("the button".to_string()))
//                .and_then(move |api: Rc<DAppApi>| {
//                    info!("application initialized, calling {:?}", callee_profile_id);
//                    api.call(&callee_profile_id, AppMessageFrame(vec![]))
//                })
                .map_err(|err| {
                    error!("call failed: {:?}", err);
                    ()
                })
                .and_then(|call: Call| {
                    info!("call accepted, waiting for incoming messages");
                    call.receiver
                        .for_each(|msg: Result<AppMessageFrame, String>| {
                            match msg {
                                Ok(frame) => {
                                    info!("got message {:?}", frame); 
                                    Ok(())
                                },
                                Err(errmsg) => {
                                    warn!("got error {:?}", errmsg); 
                                    Err(())
                                }
                            }
                        })                        
                })
                .map_err(|_err| std::io::Error::new(std::io::ErrorKind::Other, "encountered error"));
        Box::new(f)
    }
}

