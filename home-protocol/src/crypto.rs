use crate::*;
use keyvault::{PrivateKey as KeyVaultPrivateKey, PublicKey as KeyVaultPublicKey};
use osg::model::PrivateKey;

/// Something that can sign data, but cannot give out the private key.
/// Usually implemented using a private key internally, but also enables hardware wallets.
pub trait Signer {
    fn profile_id(&self) -> ProfileId;
    fn public_key(&self) -> PublicKey;
    fn sign(&self, data: &[u8]) -> Signature;
}

pub trait ProfileValidator {
    fn validate_profile(
        &self,
        public_key: &PublicKey,
        profile_id: &ProfileId,
    ) -> Result<bool, Error>;
}

impl Default for Box<ProfileValidator> {
    fn default() -> Self {
        Box::new(MultiHashProfileValidator::default())
    }
}

pub trait SignatureValidator {
    // TODO this probably should just return bool instead of Result<bool,E>
    fn validate_signature(
        &self,
        public_key: &PublicKey,
        data: &[u8],
        signature: &Signature,
    ) -> Result<bool, Error>;
}

impl Default for Box<SignatureValidator> {
    fn default() -> Self {
        Box::new(PublicKeyValidator::default())
    }
}

pub trait Validator: ProfileValidator + SignatureValidator {
    fn validate_half_proof(
        &self,
        half_proof: &RelationHalfProof,
        signer_pubkey: &PublicKey,
    ) -> Result<(), Error> {
        self.validate_signature(
            signer_pubkey,
            &RelationSignablePart::from(half_proof).serialized(),
            &half_proof.signature,
        )?;
        Ok(())
    }

    fn validate_relation_proof(
        &self,
        relation_proof: &RelationProof,
        id_1: &ProfileId,
        public_key_1: &PublicKey,
        id_2: &ProfileId,
        public_key_2: &PublicKey,
    ) -> Result<(), Error> {
        // TODO consider inverting relation_type for different directions
        let signable_a = RelationSignablePart::new(
            &relation_proof.relation_type,
            &relation_proof.a_id,
            &relation_proof.b_id,
        )
        .serialized();

        let signable_b = RelationSignablePart::new(
            &relation_proof.relation_type,
            &relation_proof.b_id,
            &relation_proof.a_id,
        )
        .serialized();

        let peer_of_id_1 = relation_proof.peer_id(&id_1)?;
        if peer_of_id_1 != id_2 {
            Err(ErrorKind::RelationValidationFailed)?
        }

        if *peer_of_id_1 == relation_proof.b_id {
            // id_1 is 'proof.id_a'
            self.validate_signature(&public_key_1, &signable_a, &relation_proof.a_signature)?;
            self.validate_signature(&public_key_2, &signable_b, &relation_proof.b_signature)?;
        } else {
            // id_1 is 'proof.id_b'
            self.validate_signature(&public_key_1, &signable_b, &relation_proof.b_signature)?;
            self.validate_signature(&public_key_2, &signable_a, &relation_proof.a_signature)?;
        }

        Ok(())
    }
}

pub struct MultiHashProfileValidator {}

impl Default for MultiHashProfileValidator {
    fn default() -> Self {
        Self {}
    }
}

impl ProfileValidator for MultiHashProfileValidator {
    fn validate_profile(
        &self,
        public_key: &PublicKey,
        profile_id: &ProfileId,
    ) -> Result<bool, Error> {
        Ok(public_key.key_id() == *profile_id)
    }
}

pub struct PrivateKeySigner {
    private_key: PrivateKey,
}

impl PrivateKeySigner {
    pub fn new(private_key: PrivateKey) -> Result<Self, Error> {
        Ok(Self { private_key })
    }
}

impl Signer for PrivateKeySigner {
    fn profile_id(&self) -> ProfileId {
        self.public_key().key_id()
    }
    fn public_key(&self) -> PublicKey {
        self.private_key.public_key()
    }
    fn sign(&self, data: &[u8]) -> Signature {
        self.private_key.sign(data)
    }
}

pub struct PublicKeyValidator {}

impl Default for PublicKeyValidator {
    fn default() -> Self {
        Self {}
    }
}

impl SignatureValidator for PublicKeyValidator {
    fn validate_signature(
        &self,
        public_key: &PublicKey,
        data: &[u8],
        signature: &Signature,
    ) -> Result<bool, Error> {
        Ok(public_key.verify(data, signature))
    }
}

#[derive(Default)]
pub struct CompositeValidator {
    profile_validator: Box<ProfileValidator>,
    signature_validator: Box<SignatureValidator>,
}

impl CompositeValidator {
    pub fn compose(
        profile_validator: Box<ProfileValidator>,
        signature_validator: Box<SignatureValidator>,
    ) -> Self {
        Self { profile_validator, signature_validator }
    }
}

impl ProfileValidator for CompositeValidator {
    fn validate_profile(
        &self,
        public_key: &PublicKey,
        profile_id: &ProfileId,
    ) -> Result<bool, Error> {
        self.profile_validator.validate_profile(public_key, profile_id)
    }
}

impl SignatureValidator for CompositeValidator {
    fn validate_signature(
        &self,
        public_key: &PublicKey,
        data: &[u8],
        signature: &Signature,
    ) -> Result<bool, Error> {
        self.signature_validator.validate_signature(public_key, data, signature)
    }
}

impl Validator for CompositeValidator {}
