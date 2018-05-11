use std::error::Error;

use multihash;
use signatory::{ed25519::FromSeed, providers::dalek};

use mercury_home_protocol::*;



#[derive(PartialEq, Eq, Clone, Debug)]
pub struct PrivateKey(pub Vec<u8>);



pub struct Ed25519Signer
{
    profile_id: ProfileId,
    public_key: PublicKey,
    signer:     dalek::Ed25519Signer,
}

impl Ed25519Signer
{
    fn new(private_key: &PrivateKey, public_key: &PublicKey) -> Result<Self, ErrorToBeSpecified>
    {
        let profile_hash = multihash::encode( multihash::Hash::Keccak256, public_key.0.as_ref() )
            .map_err( |e| ErrorToBeSpecified::TODO( e.description().to_owned() ) )?;
        let signer = dalek::Ed25519Signer::from_seed( private_key.0.as_slice() )
            .map_err( |e| ErrorToBeSpecified::TODO( e.description().to_owned() ) )?;
        Ok( Self{ public_key: public_key.to_owned(), profile_id: ProfileId(profile_hash),
                  signer: signer } )
    }
}

impl Signer for Ed25519Signer
{
    fn prof_id(&self) -> &ProfileId { &self.profile_id }
    fn pub_key(&self) -> &PublicKey { &self.public_key }

    fn sign(&self, data: &[u8]) -> Signature
    {
        use signatory::ed25519::Signer;
        // self.signer.
        Signature(Vec::new()) // TODO
    }
}


pub struct Ed25519Validator {}

impl Ed25519Validator {}

impl ProfileValidator for Ed25519Validator
{
    fn validate_profile(&self, public_key: &PublicKey, profile_id: &ProfileId)
        -> Result<bool, ErrorToBeSpecified>
    {
        Ok(false) // TODO
    }

    fn validate_signature(&self, public_key: &PublicKey, data: &[u8], signature: &Signature)
        -> Result<bool, ErrorToBeSpecified>
    {
        Ok(false) // TODO
    }
}
