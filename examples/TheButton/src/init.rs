// NOTE Though this is a full-fledged sample application, running it assumes
//      a Prometheus environment properly set up in advance,
//      e.g. a persona initialized and.registered to a home node.
//      This file contains such previous-phase setup code that probably should be moved
use std::cell::RefCell;

use multiaddr::ToMultiaddr;
use tokio_core::reactor;

use super::*;
use prometheus::dapp::user_interactor::UserInteractor;
use prometheus::home::{connection::ConnectionFactory, net::TcpHomeConnector};

pub fn ensure_registered_to_home(
    reactor: &mut reactor::Core,
    private_profilekey: PrivateKey,
    home_addr: &SocketAddr,
    app_context: &AppContext,
) -> Fallible<()> {
    use claims::repo::InMemoryProfileRepository;

    let my_signer = Rc::new(crypto::PrivateKeySigner::new(private_profilekey)?);
    let my_profile_id = my_signer.profile_id().to_owned();

    info!("dApp public key: {}", my_signer.public_key());
    info!("dApp profile id: {}", my_profile_id);

    //let my_profile = Profile::new(my_signer.public_key(), 1, vec![], Default::default());
    let profile_repo = Rc::new(RefCell::new(InMemoryProfileRepository::new()));
    let home_connector =
        Rc::new(TcpHomeConnector::new(app_context.handle.to_owned(), profile_repo));
    let conn_factory = ConnectionFactory::new(home_connector, my_signer);

    let home_conn_fut = conn_factory.open(&app_context.home_id, Some(home_addr.to_multiaddr()?));
    let home_conn = reactor.run(home_conn_fut)?;

    let reg_fut = home_conn.register();
    reactor.run(reg_fut)
}

pub fn init_publisher(_server: &Server) -> AsyncFallible<()> {
    //    let handle = server.appctx.handle.clone();
    //    let fut = init_app_common(&server.appctx)
    //        .and_then(move |my_profile: Rc<dyn MyProfile>| {
    //            my_profile.login().map(|session| (my_profile, session))
    //        })
    //        .and_then(move |(my_profile, session)| {
    //            debug!("dApp server session is ready, waiting for incoming events");
    //            handle.spawn(session.events().for_each(move |event| {
    //                debug!("dApp server received event: {:?}", event);
    //                match event {
    //                    ProfileEvent::PairingRequest(half_proof) => {
    //                        let accept_fut = my_profile
    //                            .accept_relation(&half_proof)
    //                            .map(|_proof| ())
    //                            .map_err(|e| debug!("Failed to accept pairing request: {}", e));
    //                        Box::new(accept_fut) as AsyncResult<_, _>
    //                    }
    //                    err => Box::new(Ok(debug!("Got event {:?}, ignoring it", err)).into_future()),
    //                }
    //            }));
    //            Ok(())
    //        });
    //    Box::new(fut)
    Box::new(futures::future::ok(()))
}
