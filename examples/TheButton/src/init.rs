// TODO that this file contains initialization code for the Mercury Connect Service
//      that will not be part of this program. Instead, it will run in a separated,
//      protected background service and communicate with dApps through IPC.
//      However, until it's properly implemented, dApps have to contain and instantiate it.
//use log::*;

use super::*;
//use mercury_home_protocol::keyvault::PublicKey as KeyVaultPublicKey;

pub fn init_app_common(_app_context: &AppContext) -> AsyncFallible<()> {
    //    let client_id = app_context.dapp_profile_id.clone();
    //    let home_id = app_context.home_id.clone();
    //    let init_fut = app_context
    //        .dapp_service
    //        .admin_session(None)
    //        .inspect(|_admin| debug!("Admin endpoint was connected"))
    //        .and_then(move |admin| admin.profile(client_id))
    //        .and_then(move |my_profile| my_profile.join_home(home_id).map(|()| my_profile))
    //        .inspect(|_| debug!("Successfully registered to home"))
    //        .map_err(|e| {
    //            debug!("Failed to register: {:?}", e);
    //            e
    //        });
    //    Box::new(init_fut)
    Box::new(futures::future::ok(()))
}

pub fn init_server(_server: &Server) -> AsyncFallible<()> {
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
