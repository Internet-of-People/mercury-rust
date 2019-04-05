use std::{cell::RefCell, rc::Rc};

use rand::rngs::OsRng;
use tokio_core::reactor;

use mercury_home_node::server::HomeServer;
use mercury_home_protocol::crypto::*;
use mercury_home_protocol::*;
use mercury_storage::asynch::imp::InMemoryStore;

#[cfg(test)]
pub mod connect;
#[cfg(test)]
pub mod home;

pub fn generate_keypair() -> (PrivateKey, PublicKey) {
    let mut csprng: OsRng = OsRng::new().unwrap();
    let keypair = ed25519_dalek::Keypair::generate::<sha2::Sha512, _>(&mut csprng);
    (PrivateKey::from(keypair.secret), PublicKey::from(keypair.public))
}

pub fn generate_ownprofile(
    facet: ProfileFacet,
    private_data: Vec<u8>,
) -> (OwnProfile, Ed25519Signer) {
    let (private_key, _public_key) = generate_keypair();
    let signer = Ed25519Signer::new(&private_key).expect("TODO: this should not be able to fail");
    let profile = Profile::new(&signer.profile_id(), &signer.public_key(), &facet);
    let own_profile = OwnProfile::new(&profile, &private_data);
    (own_profile, signer)
}

pub fn generate_profile(facet: ProfileFacet) -> (Profile, Ed25519Signer) {
    let (own_profile, signer) = generate_ownprofile(facet, vec![]);
    (own_profile.profile, signer)
}

pub fn generate_persona() -> (OwnProfile, Ed25519Signer) {
    let persona_facet = ProfileFacet::Persona(PersonaFacet { homes: vec![], data: Vec::new() });
    generate_ownprofile(persona_facet, vec![])
}

pub fn generate_home() -> (Profile, Ed25519Signer) {
    let home_facet = ProfileFacet::Home(HomeFacet { addrs: vec![], data: Vec::new() });
    generate_profile(home_facet)
}

pub fn default_home_server(handle: &reactor::Handle) -> HomeServer {
    HomeServer::new(
        handle,
        Rc::new(CompositeValidator::default()),
        Rc::new(RefCell::new(InMemoryStore::new())),
        Rc::new(RefCell::new(InMemoryStore::new())),
    )
}

pub fn first_home_of(own_profile: &OwnProfile) -> &RelationProof {
    match own_profile.profile.facet {
        ProfileFacet::Persona(ref persona) => persona.homes.get(0).unwrap(),
        _ => panic!("Profile is not a persona, no home found"),
    }
}
