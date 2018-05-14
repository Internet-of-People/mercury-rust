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
        let profile_hash = multihash::encode( multihash::Hash::Keccak256, public_key.0.as_slice() )
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
        let signature = self.signer.sign(data)
            .unwrap(); // TODO ERROR HANDLING how to handle possibly returned errors here?
        let signature_bytes: Box<[u8]> = Box::new(signature.0);
        Signature( signature_bytes.into() )
    }
}


pub struct Ed25519Validator {}

impl Ed25519Validator
{
    pub fn new() -> Self { Self{} }
}

impl ProfileValidator for Ed25519Validator
{
    fn validate_profile(&self, public_key: &PublicKey, profile_id: &ProfileId)
        -> Result<bool, ErrorToBeSpecified>
    {
        let profile_hash = multihash::encode( multihash::Hash::Keccak256, public_key.0.as_slice() )
            .map_err( |e| ErrorToBeSpecified::TODO( e.description().to_owned() ) )?;
        Ok( profile_hash == profile_id.0 )
    }

    fn validate_signature(&self, public_key: &PublicKey, data: &[u8], signature: &Signature)
        -> Result<bool, ErrorToBeSpecified>
    {
        use signatory::ed25519::{DefaultVerifier, Verifier};
        let pubkey = ::signatory::ed25519::PublicKey::from_bytes( public_key.0.as_slice() )
            .map_err( |e| ErrorToBeSpecified::TODO( e.description().to_owned() ) )?;
        let signo = ::signatory::ed25519::Signature::from_bytes( signature.0.as_slice() )
            .map_err( |e| ErrorToBeSpecified::TODO( e.description().to_owned() ) )?;
        DefaultVerifier::verify(&pubkey, data, &signo)
            .map( |()| true )
            .map_err( |e| ErrorToBeSpecified::TODO( e.description().to_owned() ) )
            // TODO hwo to determine when to return Ok(false) here, i.e. signature does not match but validation was otherwise successful
    }
}
