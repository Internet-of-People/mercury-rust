use crate::secp256k1::Network;

/// Strategies for the BTC, BCC and BSV main (production) network.
pub struct Mainnet;

impl Network for Mainnet {
    fn p2pkh_addr(&self) -> &'static [u8; 1] {
        b"\x00"
    }
    fn p2sh_addr(&self) -> &'static [u8; 1] {
        b"\x05"
    }
    fn wif(&self) -> &'static [u8; 1] {
        b"\x80"
    }
    fn bip32_xpub(&self) -> &'static [u8; 4] {
        b"\x04\x88\xB2\x1E"
    }
    fn bip32_xprv(&self) -> &'static [u8; 4] {
        b"\x04\x88\xAD\xE4"
    }
    fn message_prefix(&self) -> &'static str {
        "\x18Bitcoin Signed Message:\n"
    }
    fn slip44(&self) -> i32 {
        0
    }
}

/// Strategies for the BTC, BCC and BSV test (staging) network.
pub struct Testnet;

impl Network for Testnet {
    fn p2pkh_addr(&self) -> &'static [u8; 1] {
        b"\x6F"
    }
    fn p2sh_addr(&self) -> &'static [u8; 1] {
        b"\xC4"
    }
    fn wif(&self) -> &'static [u8; 1] {
        b"\xEF"
    }
    fn bip32_xpub(&self) -> &'static [u8; 4] {
        b"\x04\x35\x87\xCF"
    }
    fn bip32_xprv(&self) -> &'static [u8; 4] {
        b"\x04\x35\x83\x94"
    }
    fn message_prefix(&self) -> &'static str {
        "\x18Bitcoin Signed Message:\n"
    }
    fn slip44(&self) -> i32 {
        1
    }
}
