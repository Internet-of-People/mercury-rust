use super::*;

erased_type! {
    /// Type-erased [`PrivateKey`]
    ///
    /// [`PrivateKey`]: ../trait.AsymmetricCrypto.html#associatedtype.PrivateKey
    #[derive(Debug)]
    pub struct MPrivateKey {}
}

macro_rules! public_key {
    ($suite:ident, $self_:tt) => {{
        let result = reify!($suite, sk, $self_).public_key();
        erase!($suite, MPublicKey, result)
    }};
}

macro_rules! sign {
    ($suite:ident, $self_:tt, $data:ident) => {{
        let result = reify!($suite, sk, $self_).sign($data);
        erase!($suite, MSignature, result)
    }};
}

impl PrivateKey<MultiCipher> for MPrivateKey {
    fn public_key(&self) -> MPublicKey {
        visit!(public_key(self))
    }
    fn sign<D: AsRef<[u8]>>(&self, data: D) -> MSignature {
        visit!(sign(self, data))
    }
}

impl From<EdPrivateKey> for MPrivateKey {
    fn from(src: EdPrivateKey) -> Self {
        erase!(e, MPrivateKey, src)
    }
}
