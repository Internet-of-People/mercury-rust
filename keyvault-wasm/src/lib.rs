use wasm_bindgen::prelude::*;

use keyvault::multicipher;
use keyvault::PublicKey as KeyVaultPublicKey;

// NOTE Always receive function arguments as references (as long as bindgen allows)
//      and return results by value. Otherwise the generated code may destroy
//      JS variables by moving out underlying pointers
//      (at least in your custom structs like SignedMessage below).

pub fn err_to_js<E: ToString>(e: E) -> JsValue {
    JsValue::from(e.to_string())
}

#[wasm_bindgen(js_name = KeyId)]
#[derive(Clone, Debug)]
pub struct JsKeyId {
    inner: multicipher::MKeyId,
}

#[wasm_bindgen(js_class = KeyId)]
impl JsKeyId {
    #[wasm_bindgen(constructor)]
    pub fn new(key_id_str: &str) -> Result<JsKeyId, JsValue> {
        let inner: multicipher::MKeyId = key_id_str.parse().map_err(err_to_js)?;
        Ok(Self { inner })
    }

    #[wasm_bindgen]
    pub fn prefix() -> String {
        multicipher::MKeyId::PREFIX.to_string()
    }

    #[wasm_bindgen(js_name = toString)]
    pub fn to_string(&self) -> String {
        self.inner.to_string()
    }
}

impl From<multicipher::MKeyId> for JsKeyId {
    fn from(inner: multicipher::MKeyId) -> Self {
        Self { inner }
    }
}

#[wasm_bindgen(js_name = PublicKey)]
#[derive(Clone, Debug)]
pub struct JsPublicKey {
    inner: multicipher::MPublicKey,
}

#[wasm_bindgen(js_class = PublicKey)]
impl JsPublicKey {
    #[wasm_bindgen(constructor)]
    pub fn new(pub_key_str: &str) -> Result<JsPublicKey, JsValue> {
        let inner: multicipher::MPublicKey = pub_key_str.parse().map_err(err_to_js)?;
        Ok(Self { inner })
    }

    #[wasm_bindgen]
    pub fn prefix() -> String {
        multicipher::MPublicKey::PREFIX.to_string()
    }

    #[wasm_bindgen(js_name = keyId)]
    pub fn key_id(&self) -> JsKeyId {
        JsKeyId { inner: self.inner.key_id() }
    }

    #[wasm_bindgen(js_name = validateId)]
    pub fn validate_id(&self, key_id: &JsKeyId) -> bool {
        self.inner.validate_id(&key_id.inner)
    }

    #[wasm_bindgen(js_name = toString)]
    pub fn to_string(&self) -> String {
        self.inner.to_string()
    }
}

impl From<multicipher::MPublicKey> for JsPublicKey {
    fn from(inner: multicipher::MPublicKey) -> Self {
        Self { inner }
    }
}

#[wasm_bindgen(js_name = Signature)]
#[derive(Clone, Debug)]
pub struct JsSignature {
    inner: multicipher::MSignature,
}

#[wasm_bindgen(js_class = Signature)]
impl JsSignature {
    #[wasm_bindgen(constructor)]
    pub fn new(sign_str: &str) -> Result<JsSignature, JsValue> {
        let inner: multicipher::MSignature = sign_str.parse().map_err(err_to_js)?;
        Ok(Self { inner })
    }

    #[wasm_bindgen]
    pub fn prefix() -> String {
        multicipher::MSignature::PREFIX.to_string()
    }

    #[wasm_bindgen(js_name = toString)]
    pub fn to_string(&self) -> String {
        self.inner.to_string()
    }
}

impl From<multicipher::MSignature> for JsSignature {
    fn from(inner: multicipher::MSignature) -> Self {
        Self { inner }
    }
}

pub trait Wraps<T>: From<T> {
    fn inner(&self) -> &T;
}

impl Wraps<multicipher::MKeyId> for JsKeyId {
    fn inner(&self) -> &multicipher::MKeyId {
        &self.inner
    }
}

impl Wraps<multicipher::MPublicKey> for JsPublicKey {
    fn inner(&self) -> &multicipher::MPublicKey {
        &self.inner
    }
}

impl Wraps<multicipher::MSignature> for JsSignature {
    fn inner(&self) -> &multicipher::MSignature {
        &self.inner
    }
}
