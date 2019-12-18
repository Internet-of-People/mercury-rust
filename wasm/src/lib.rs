use wasm_bindgen::prelude::*;

use did::vault::ProfileVault;
use keyvault::PublicKey as KeyVaultPublicKey;

// NOTE Always receive function arguments as references (as long as bindgen allows)
//      and return results by value. Otherwise the generated code may destroy
//      JS variables by moving out underlying pointers
//      (at least in your custom structs like SignedMessage below).

fn err_to_js<E: ToString>(e: E) -> JsValue {
    JsValue::from(e.to_string())
}

#[wasm_bindgen]
pub struct Vault {
    inner: did::vault::HdProfileVault,
}

#[wasm_bindgen]
#[derive(Clone, Debug)]
pub struct KeyId {
    inner: keyvault::multicipher::MKeyId,
}

#[wasm_bindgen]
impl KeyId {
    #[wasm_bindgen(constructor)]
    pub fn new(key_id_str: &str) -> Result<KeyId, JsValue> {
        let inner: did::ProfileId = key_id_str.parse().map_err(err_to_js)?;
        Ok(Self { inner })
    }

    #[wasm_bindgen(js_name = toString)]
    pub fn to_string(&self) -> String {
        self.inner.to_string()
    }
}

#[wasm_bindgen]
#[derive(Clone, Debug)]
pub struct PublicKey {
    inner: keyvault::multicipher::MPublicKey,
}

#[wasm_bindgen]
impl PublicKey {
    #[wasm_bindgen(constructor)]
    pub fn new(pub_key_str: &str) -> Result<PublicKey, JsValue> {
        let inner: did::PublicKey = pub_key_str.parse().map_err(err_to_js)?;
        Ok(Self { inner })
    }

    #[wasm_bindgen(js_name = keyId)]
    pub fn key_id(&self) -> KeyId {
        KeyId { inner: self.inner.key_id() }
    }

    #[wasm_bindgen(js_name = validateId)]
    pub fn validate_id(&self, key_id: &KeyId) -> bool {
        self.inner.validate_id(&key_id.inner)
    }

    #[wasm_bindgen(js_name = toString)]
    pub fn to_string(&self) -> String {
        self.inner.to_string()
    }
}

#[wasm_bindgen]
#[derive(Clone, Debug)]
pub struct Signature {
    inner: keyvault::multicipher::MSignature,
}

#[wasm_bindgen]
impl Signature {
    #[wasm_bindgen(constructor)]
    pub fn new(sign_str: &str) -> Result<Signature, JsValue> {
        let inner: did::Signature = sign_str.parse().map_err(err_to_js)?;
        Ok(Self { inner })
    }

    #[wasm_bindgen(js_name = toString)]
    pub fn to_string(&self) -> String {
        self.inner.to_string()
    }
}

#[wasm_bindgen]
pub struct SignedMessage {
    public_key: PublicKey,
    message: Box<[u8]>,
    signature: Signature,
}

#[wasm_bindgen]
impl SignedMessage {
    #[wasm_bindgen(constructor)]
    pub fn new(public_key: &PublicKey, message: &[u8], signature: &Signature) -> Self {
        Self {
            public_key: public_key.to_owned(),
            message: message.to_owned().into_boxed_slice(),
            signature: signature.to_owned(),
        }
    }

    #[wasm_bindgen(getter, js_name = publicKey)]
    pub fn public_key(&self) -> PublicKey {
        self.public_key.to_owned()
    }

    #[wasm_bindgen(getter)]
    pub fn message(&self) -> Box<[u8]> {
        self.message.clone()
    }

    #[wasm_bindgen(getter)]
    pub fn signature(&self) -> Signature {
        self.signature.to_owned()
    }

    pub fn validate(&self) -> bool {
        self.public_key.inner.verify(&self.message, &self.signature.inner)
    }

    #[wasm_bindgen(js_name = validateWithId)]
    pub fn validate_with_id(&self, signer_id: &KeyId) -> bool {
        self.public_key.validate_id(signer_id) && self.validate()
    }
}

#[wasm_bindgen]
impl Vault {
    #[wasm_bindgen(constructor)]
    pub fn new(seed_phrase: &str) -> Result<Vault, JsValue> {
        let seed = keyvault::Seed::from_bip39(seed_phrase).map_err(err_to_js)?;
        let vault = did::vault::HdProfileVault::create(seed);
        Ok(Self { inner: vault })
    }

    pub fn serialize(&self) -> Result<String, JsValue> {
        serde_json::to_string_pretty(&self.inner).map_err(err_to_js)
    }

    pub fn deserialize(from: &str) -> Result<Vault, JsValue> {
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
    pub fn active_id(&self) -> Result<Option<KeyId>, JsValue> {
        let active_id = self.inner.get_active().map_err(err_to_js)?;
        let active_str = active_id.map(|id| KeyId { inner: id });
        Ok(active_str)
    }

    #[wasm_bindgen(js_name = createId)]
    pub fn create_id(&mut self) -> Result<KeyId, JsValue> {
        let key = self.inner.create_key(None).map_err(err_to_js)?;
        Ok(KeyId { inner: key.key_id() })
    }

    pub fn sign(&self, key_id: &KeyId, message: &[u8]) -> Result<SignedMessage, JsValue> {
        let signed_message = self.inner.sign(&key_id.inner, message).map_err(err_to_js)?;

        let result = SignedMessage {
            public_key: PublicKey { inner: signed_message.public_key().to_owned() },
            message: signed_message.message().to_owned().into_boxed_slice(),
            signature: Signature { inner: signed_message.signature().to_owned() },
        };
        Ok(result)
    }
}
