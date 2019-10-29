use crate::*;
use keyvault::PublicKey as KeyVaultPublicKey;

pub trait ProfileIdValidator {
    fn validate_profile_auth(
        &self,
        public_key: &PublicKey,
        profile_id: &ProfileId,
    ) -> Result<(), Error>;
}

impl Default for Box<dyn ProfileIdValidator> {
    fn default() -> Self {
        Box::new(MultiHashProfileValidator::default())
    }
}

pub trait SignatureValidator {
    fn validate_signature(
        &self,
        public_key: &PublicKey,
        // TODO add here: profile_auth: ProfileAuthData,
        data: &[u8],
        signature: &Signature,
    ) -> Result<(), Error>;
}

impl Default for Box<dyn SignatureValidator> {
    fn default() -> Self {
        Box::new(PublicKeyValidator::default())
    }
}

pub trait Validator: ProfileIdValidator + SignatureValidator {
    fn validate_half_proof(
        &self,
        half_proof: &RelationHalfProof,
        signer_pubkey: &PublicKey,
    ) -> Result<(), Error> {
        let signable = RelationSignablePart::from(half_proof).serialized();
        self.validate_signature(signer_pubkey, &signable, &half_proof.signature)
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

        if peer_of_id_1 == &relation_proof.b_id {
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

impl ProfileIdValidator for MultiHashProfileValidator {
    fn validate_profile_auth(
        &self,
        public_key: &PublicKey,
        profile_id: &ProfileId,
    ) -> Result<(), Error> {
        if public_key.validate_id(profile_id) {
            Ok(())
        } else {
            Err(ErrorKind::ProfileValidationFailed.into())
        }
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
    ) -> Result<(), Error> {
        if public_key.verify(data, signature) {
            Ok(())
        } else {
            Err(ErrorKind::InvalidSignature.into())
        }
    }
}

#[derive(Default)]
pub struct CompositeValidator {
    profile_validator: Box<dyn ProfileIdValidator>,
    signature_validator: Box<dyn SignatureValidator>,
}

impl CompositeValidator {
    pub fn compose(
        profile_validator: Box<dyn ProfileIdValidator>,
        signature_validator: Box<dyn SignatureValidator>,
    ) -> Self {
        Self { profile_validator, signature_validator }
    }
}

impl ProfileIdValidator for CompositeValidator {
    fn validate_profile_auth(
        &self,
        public_key: &PublicKey,
        profile_id: &ProfileId,
    ) -> Result<(), Error> {
        self.profile_validator.validate_profile_auth(public_key, profile_id)
    }
}

impl SignatureValidator for CompositeValidator {
    fn validate_signature(
        &self,
        public_key: &PublicKey,
        data: &[u8],
        signature: &Signature,
    ) -> Result<(), Error> {
        self.signature_validator.validate_signature(public_key, data, signature)
    }
}

impl Validator for CompositeValidator {}
