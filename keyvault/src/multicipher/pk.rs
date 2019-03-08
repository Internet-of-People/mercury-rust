use super::*;

erased_type! {
    /// Type-erased [`PublicKey`]
    ///
    /// [`PublicKey`]: ../trait.AsymmetricCrypto.html#associatedtype.PublicKey
    #[derive(Debug)]
    pub struct MPublicKey {}
}

macro_rules! key_id {
    ($suite:ident, $self_:tt) => {{
        let result = reify!($suite, pk, $self_).key_id();
        erase!($suite, MKeyId, result)
    }};
}

macro_rules! verify {
    ($suite:ident, $self_:tt, $data:ident, $sig:ident) => {
        reify!($suite, pk, $self_).verify($data, reify!($suite, sig, $sig))
    };
}

impl PublicKey<MultiCipher> for MPublicKey {
    fn key_id(&self) -> MKeyId {
        visit!(key_id(self))
    }
    fn verify<D: AsRef<[u8]>>(&self, data: D, sig: &MSignature) -> bool {
        if self.suite != sig.suite {
            return false;
        }
        visit!(verify(self, data, sig))
    }
}
