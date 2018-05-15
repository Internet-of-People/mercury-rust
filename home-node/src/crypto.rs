use std::error::Error;

use multihash;
use signatory::{ed25519::FromSeed, providers::dalek};

use mercury_home_protocol::*;



pub trait ProfileValidator
{
    fn validate_profile(&self, public_key: &PublicKey, profile_id: &ProfileId)
        -> Result<bool, ErrorToBeSpecified>;
}

pub trait SignatureValidator
{
    fn validate_signature(&self, public_key: &PublicKey, data: &[u8], signature: &Signature)
        -> Result<bool, ErrorToBeSpecified>;
}


pub struct CompositeValidator<P,S>
{
    profile_validator:      P,
    signature_validator:    S,
}

impl<P,S> CompositeValidator<P,S>
{
    pub fn new(profile_validator: P, signature_validator: S) -> Self
        { Self{ profile_validator: profile_validator, signature_validator: signature_validator } }
}

impl<P: ProfileValidator, S: SignatureValidator> Validator for CompositeValidator<P,S>
{
    fn validate_profile(&self, public_key: &PublicKey, profile_id: &ProfileId)
        -> Result<bool, ErrorToBeSpecified>
    { self.profile_validator.validate_profile(public_key, profile_id) }

    fn validate_signature(&self, public_key: &PublicKey, data: &[u8], signature: &Signature)
        -> Result<bool, ErrorToBeSpecified>
    { self.signature_validator.validate_signature(public_key, data, signature) }
}



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
    pub fn new(private_key: &PrivateKey, public_key: &PublicKey) -> Result<Self, ErrorToBeSpecified>
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

impl SignatureValidator for Ed25519Validator
{
    fn validate_signature(&self, public_key: &PublicKey, data: &[u8], signature: &Signature)
        -> Result<bool, ErrorToBeSpecified>
    {
        use signatory::ed25519::{DefaultVerifier, Verifier};
        let pubkey = ::signatory::ed25519::PublicKey::from_bytes( public_key.0.as_slice() )
            .map_err( |e| ErrorToBeSpecified::TODO( e.description().to_owned() ) )?;
        let signo = ::signatory::ed25519::Signature::from_bytes( signature.0.as_slice() )
            .map_err( |e| ErrorToBeSpecified::TODO( e.description().to_owned() ) )?;
        DefaultVerifier::verify(&pubkey, data, &signo)
            // TODO hwo to determine when to return Ok(false) here, i.e. signature does not match but validation was otherwise successful
            .map( |()| true )
            .map_err( |e| ErrorToBeSpecified::TODO( e.description().to_owned() ) )
    }
}


pub struct MultiHashProfileValidator {}

impl MultiHashProfileValidator
{
    pub fn new() -> Self { Self{} }
}

impl ProfileValidator for MultiHashProfileValidator
{
    fn validate_profile(&self, public_key: &PublicKey, profile_id: &ProfileId)
        -> Result<bool, ErrorToBeSpecified>
    {
        let id_hashalgo = multihash::decode(profile_id.0.as_slice())
            .map_err(|e| ErrorToBeSpecified::TODO(e.description().to_owned()))
            ?.alg;
        let key_hash = multihash::encode(id_hashalgo, public_key.0.as_slice())
            .map_err(|e| ErrorToBeSpecified::TODO(e.description().to_owned()))?;
        Ok(key_hash == profile_id.0)
    }
}


#[cfg(test)]
mod tests
{
    use super::*;

    #[test]
    fn test_ed25519()
    {
        let secret_key = PrivateKey( b"\x83\x3F\xE6\x24\x09\x23\x7B\x9D\x62\xEC\x77\x58\x75\x20\x91\x1E\x9A\x75\x9C\xEC\x1D\x19\x75\x5B\x7D\xA9\x01\xB9\x6D\xCA\x3D\x42".to_vec() );
        let public_key = PublicKey( b"\xEC\x17\x2B\x93\xAD\x5E\x56\x3B\xF4\x93\x2C\x70\xE1\x24\x50\x34\xC3\x54\x67\xEF\x2E\xFD\x4D\x64\xEB\xF8\x19\x68\x34\x67\xE2\xBF".to_vec() );
        let message = b"\xDD\xAF\x35\xA1\x93\x61\x7A\xBA\xCC\x41\x73\x49\xAE\x20\x41\x31\x12\xE6\xFA\x4E\x89\xA9\x7E\xA2\x0A\x9E\xEE\xE6\x4B\x55\xD3\x9A\x21\x92\x99\x2A\x27\x4F\xC1\xA8\x36\xBA\x3C\x23\xA3\xFE\xEB\xBD\x45\x4D\x44\x23\x64\x3C\xE8\x0E\x2A\x9A\xC9\x4F\xA5\x4C\xA4\x9F";

        let signer = Ed25519Signer::new(&secret_key, &public_key).unwrap();
        let signature = signer.sign(message);
        let expected_signature = b"\xDC\x2A\x44\x59\xE7\x36\x96\x33\xA5\x2B\x1B\xF2\x77\x83\x9A\x00\x20\x10\x09\xA3\xEF\xBF\x3E\xCB\x69\xBE\xA2\x18\x6C\x26\xB5\x89\x09\x35\x1F\xC9\xAC\x90\xB3\xEC\xFD\xFB\xC7\xC6\x64\x31\xE0\x30\x3D\xCA\x17\x9C\x13\x8A\xC1\x7A\xD9\xBE\xF1\x17\x73\x31\xA7\x04";
        assert_eq!( signature.0.as_slice(), expected_signature as &[u8] );

        let validator = Ed25519Validator{};
        let valid_res = validator.validate_signature(&public_key, message, &signature);
        assert!( valid_res.unwrap() );

        let invalid_signature = Signature( b"invalidsignature".to_vec() );
        let invalid_res = validator.validate_signature(&public_key, message, &invalid_signature);
        assert!( invalid_res.is_err() );
    }
}