//use std::rc::Rc;
//
//use futures::Future;
//use multiaddr::Multiaddr;
//
//use crate::home::net::HomeConnector;
//use mercury_home_protocol::{AsyncFallible, Home, Profile, ProfileId, Signer};
//
//pub struct ConnectionFactory {
//    connector: Rc<dyn HomeConnector>,
//    signer: Rc<dyn Signer>,
//}
//
//impl ConnectionFactory {
//    pub fn new(connector: Rc<dyn HomeConnector>, signer: Rc<dyn Signer>) -> Self {
//        Self { connector, signer }
//    }
//
//    pub fn open(
//        &self,
//        home_profile_id: &ProfileId,
//        addr_hint: Option<Multiaddr>,
//    ) -> AsyncFallible<Connection> {
//        let signer = self.signer.to_owned();
//        let open_fut = self
//            .connector
//            .clone()
//            .connect(home_profile_id, addr_hint, self.signer.to_owned())
//            .map(|home| Connection::new(home, signer))
//            .map_err(|e| e.into());
//        Box::new(open_fut)
//    }
//}
//
//pub struct Connection {
//    proxy: Rc<dyn Home>,
//    signer: Rc<dyn Signer>,
//    //connector: Rc<ConnectionFactory>,
//}
//
//impl Connection {
//    pub fn new(proxy: Rc<dyn Home>, signer: Rc<dyn Signer>) -> Self {
//        Self { proxy, signer }
//    }
//
//    pub fn register(&self) -> AsyncFallible<()> {
//        unimplemented!()
//    }
//    pub fn unregister(&self, new_home: Option<ProfileId>) -> AsyncFallible<()> {
//        unimplemented!()
//    }
//
//    pub fn backup(&self, data: &Profile) -> AsyncFallible<()> {
//        unimplemented!()
//    }
//    pub fn restore(&self) -> AsyncFallible<Profile> {
//        unimplemented!()
//    }
//
//    pub fn login(&self) -> AsyncFallible<Session> {
//        unimplemented!()
//    }
//
//    //    fn join_home(
//    //        &self,
//    //        home_id: ProfileId,
//    //        //invite: Option<HomeInvitation>,
//    //    ) -> AsyncFallible<()> {
//    //        let half_proof = RelationHalfProof::new(
//    //            RelationProof::RELATION_TYPE_HOSTED_ON_HOME,
//    //            &home_id,
//    //            &*self.signer,
//    //        );
//    //
//    //        let own_profile_cell = self.own_profile.clone();
//    //        let own_profile_dataclone = self.own_profile.borrow().to_owned();
//    //        //let profile_repo = self.profile_repo.clone();
//    //        let reg_fut = self
//    //            .connect_home(&home_id)
//    //            .and_then(move |home| {
//    //                home.register(own_profile_dataclone, half_proof) //, invite)
//    //                    .map_err(|(_own_prof, err)| err.context(ErrorKind::RegistrationFailed).into())
//    //            })
//    //            // TODO we should also notify the AdminSession here to update its profile_store
//    //            //.and_then( |own_profile| ... )
//    //            .map(move |own_profile| {
//    //                own_profile_cell.replace(own_profile.clone());
//    //                // TODO remove this after testing
//    //                //debug!("Hack: overwriting persona entry with home added: {:?}", own_profile.public_data());
//    //                //profile_repo.borrow_mut().set_public(own_profile.public_data())
//    //                //    .map_err(|e| e.context(ErrorKind::RegistrationFailed).into())
//    //            });
//    //        Box::new(reg_fut)
//    //    }
//    //
//    //    fn leave_home(&self, home_id: ProfileId, newhome_id: Option<Profile>) -> AsyncFallible<()> {
//    //        let unreg_fut = self.login_home(home_id)
//    //            .map_err(|err| err.context(ErrorKind::LoginFailed).into())
//    //            .and_then( move |my_session|
//    //                my_session.session()
//    //                    .unregister(newhome_id)
//    //                    .map_err(|err| err.context(ErrorKind::DeregistrationFailed).into())
//    //            )
//    //            // TODO we should also notify the AdminSession here to update its profile_store
//    //            // .and_then( || ... )
//    //            ;
//    //        Box::new(unreg_fut)
//    //    }
//}
//
//pub struct Session {}
