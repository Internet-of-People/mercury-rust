use wasm_bindgen::prelude::*;

use did::vault::ProfileVault;
use keyvault::PublicKey as KeyVaultPublicKey;
use keyvault_wasm::*;

#[wasm_bindgen(js_name = SignedMessage)]
pub struct JsSignedMessage {
    public_key: JsPublicKey,
    message: Box<[u8]>,
    signature: JsSignature,
}

#[wasm_bindgen(js_class = SignedMessage)]
impl JsSignedMessage {
    #[wasm_bindgen(constructor)]
    pub fn new(public_key: &JsPublicKey, message: &[u8], signature: &JsSignature) -> Self {
        Self::new_owned(
            public_key.to_owned(),
            message.to_owned().into_boxed_slice(),
            signature.to_owned(),
        )
    }

    #[wasm_bindgen(getter, js_name = publicKey)]
    pub fn public_key(&self) -> JsPublicKey {
        self.public_key.to_owned()
    }

    #[wasm_bindgen(getter)]
    pub fn message(&self) -> Box<[u8]> {
        self.message.clone()
    }

    #[wasm_bindgen(getter)]
    pub fn signature(&self) -> JsSignature {
        self.signature.to_owned()
    }

    pub fn validate(&self) -> bool {
        self.public_key.inner().verify(&self.message, &self.signature.inner())
    }

    #[wasm_bindgen(js_name = validateWithId)]
    pub fn validate_with_id(&self, signer_id: &JsKeyId) -> bool {
        self.public_key.validate_id(signer_id) && self.validate()
    }
}

impl JsSignedMessage {
    pub fn new_owned(public_key: JsPublicKey, message: Box<[u8]>, signature: JsSignature) -> Self {
        Self { public_key, message, signature }
    }
}

#[wasm_bindgen(js_name = Vault)]
pub struct JsVault {
    inner: did::vault::HdProfileVault,
}

#[wasm_bindgen(js_class = Vault)]
impl JsVault {
    #[wasm_bindgen(constructor)]
    pub fn new(seed_phrase: &str) -> Result<JsVault, JsValue> {
        let seed = keyvault::Seed::from_bip39(seed_phrase).map_err(err_to_js)?;
        let vault = did::vault::HdProfileVault::create(seed);
        Ok(Self { inner: vault })
    }

    pub fn serialize(&self) -> Result<String, JsValue> {
        serde_json::to_string_pretty(&self.inner).map_err(err_to_js)
    }

    pub fn deserialize(from: &str) -> Result<JsVault, JsValue> {
        let vault = serde_json::from_str(&from).map_err(err_to_js)?;
        Ok(Self { inner: vault })
    }

    pub fn profiles(&self) -> Result<Box<[JsValue]>, JsValue> {
        let profiles = self
            .inner
            .profiles()
            .map_err(err_to_js)?
            .iter()
            .map(|rec| JsValue::from_str(&rec.id().to_string()))
            .collect::<Vec<_>>();
        Ok(profiles.into_boxed_slice())
    }

    #[wasm_bindgen(js_name = activeId)]
    pub fn active_id(&self) -> Result<Option<JsKeyId>, JsValue> {
        let active_id = self.inner.get_active().map_err(err_to_js)?;
        let active_str = active_id.map(|id| JsKeyId::from(id));
        Ok(active_str)
    }

    #[wasm_bindgen(js_name = createId)]
    pub fn create_id(&mut self) -> Result<JsKeyId, JsValue> {
        let key = self.inner.create_key(None).map_err(err_to_js)?;
        Ok(JsKeyId::from(key.key_id()))
    }

    pub fn sign(&self, key_id: &JsKeyId, message: &[u8]) -> Result<JsSignedMessage, JsValue> {
        let signed_message = self.inner.sign(&key_id.inner(), message).map_err(err_to_js)?;

        let result = JsSignedMessage::new_owned(
            JsPublicKey::from(signed_message.public_key().to_owned()),
            signed_message.message().to_owned().into_boxed_slice(),
            JsSignature::from(signed_message.signature().to_owned()),
        );
        Ok(result)
    }
}
