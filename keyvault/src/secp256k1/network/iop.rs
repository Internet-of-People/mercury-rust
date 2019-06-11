use crate::secp256k1::Network;

/// Strategies for the IOP main (production) network.
pub struct Mainnet;

impl Network for Mainnet {
    fn p2pkh_addr(&self) -> &'static [u8; 1] {
        b"\x75"
    }
    fn p2sh_addr(&self) -> &'static [u8; 1] {
        b"\xA4"
    }
    fn wif(&self) -> &'static [u8; 1] {
        b"\x31"
    }
    fn bip32_xprv(&self) -> &'static [u8; 4] {
        b"\xAE\x34\x16\xF6"
    }
    fn bip32_xpub(&self) -> &'static [u8; 4] {
        b"\x27\x80\x91\x5F"
    }
}

/// Strategies for the BTC, BCC and BSV test (staging) network.
pub struct Testnet;

impl Network for Testnet {
    fn p2pkh_addr(&self) -> &'static [u8; 1] {
        b"\x82"
    }
    fn p2sh_addr(&self) -> &'static [u8; 1] {
        b"\x31"
    }
    fn wif(&self) -> &'static [u8; 1] {
        b"\x4C"
    }
    fn bip32_xprv(&self) -> &'static [u8; 4] {
        b"\x2B\x7F\xA4\x2A"
    }
    fn bip32_xpub(&self) -> &'static [u8; 4] {
        b"\xBB\x8F\x48\x52"
    }
}
