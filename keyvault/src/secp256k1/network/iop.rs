use crate::secp256k1::Network;

/// Strategies for the IOP main (production) network.
pub struct Mainnet;

impl Network for Mainnet {
    fn p2pkh_addr(&self) -> &'static [u8; 1] {
        b"\x75" // 115
    }
    fn p2sh_addr(&self) -> &'static [u8; 1] {
        b"\xAE" // 174
    }
    fn wif(&self) -> &'static [u8; 1] {
        b"\x31" // 49
    }
    fn bip32_xpub(&self) -> &'static [u8; 4] {
        b"\x27\x80\x91\x5F"
    }
    fn bip32_xprv(&self) -> &'static [u8; 4] {
        b"\xAE\x34\x16\xF6"
    }
    fn message_prefix(&self) -> &'static str {
        "\x18IoP Signed Message:\n"
    }
    fn slip44(&self) -> i32 {
        0x42 // 66
    }
}

/// Strategies for the BTC, BCC and BSV test (staging) network.
pub struct Testnet;

impl Network for Testnet {
    fn p2pkh_addr(&self) -> &'static [u8; 1] {
        b"\x82" // 130
    }
    fn p2sh_addr(&self) -> &'static [u8; 1] {
        b"\x31" // 49
    }
    fn wif(&self) -> &'static [u8; 1] {
        b"\x4C" // 76
    }
    fn bip32_xpub(&self) -> &'static [u8; 4] {
        b"\xBB\x8F\x48\x52"
    }
    fn bip32_xprv(&self) -> &'static [u8; 4] {
        b"\x2B\x7F\xA4\x2A"
    }
    fn message_prefix(&self) -> &'static str {
        "\x18IoP SignedMessage:\n"
    }
    fn slip44(&self) -> i32 {
        0x42 // 66
    }
}
