use std::{cell::RefCell, rc::Rc};

use rand::rngs::OsRng;
use tokio_core::reactor;

use claims::repo::InMemoryProfileRepository;
use mercury_home_node::server::HomeServer;
use mercury_home_protocol::crypto::*;
use mercury_home_protocol::*;

#[cfg(test)]
pub mod connect;
#[cfg(test)]
pub mod home;

pub fn generate_keypair() -> PrivateKey {
    let mut csprng: OsRng = OsRng::new().unwrap();
    let ed_rnd_keypair = ed25519_dalek::Keypair::generate(&mut csprng);
    PrivateKey::from(ed25519::EdPrivateKey::from(ed_rnd_keypair))
}

pub fn generate_ownprofile(attributes: AttributeMap) -> (OwnProfile, PrivateKeySigner) {
    let private_key = generate_keypair();
    let signer = PrivateKeySigner::new(private_key).expect("TODO: this should not ever fail");
    let profile = Profile::new(signer.public_key(), 1, vec![], attributes);
    let own_profile = OwnProfile::new(profile, vec![]);
    (own_profile, signer)
}

//facet: ProfileFacet
pub fn generate_profile(attributes: AttributeMap) -> (Profile, PrivateKeySigner) {
    let (own_profile, signer) = generate_ownprofile(attributes);
    (own_profile.public_data(), signer)
}

pub fn generate_persona() -> (OwnProfile, PrivateKeySigner) {
    let attributes = PersonaFacet::new(vec![], vec![]).to_attributes();
    generate_ownprofile(attributes)
}

pub fn generate_home() -> (Profile, PrivateKeySigner) {
    let attributes = HomeFacet::new(vec![], vec![]).to_attributes();
    generate_profile(attributes)
}

pub fn default_home_server(handle: &reactor::Handle) -> HomeServer {
    HomeServer::new(
        handle,
        Rc::new(CompositeValidator::default()),
        Rc::new(RefCell::new(InMemoryProfileRepository::new())),
        Rc::new(RefCell::new(InMemoryProfileRepository::new())),
    )
}

pub fn first_home_of(own_profile: &OwnProfile) -> RelationProof {
    match own_profile.public_data().as_persona() {
        Some(ref persona) => persona.homes[0].clone(),
        _ => panic!("Profile is not a persona, no home found"),
    }
}
