use crate::secp256k1::Network;

/// Strategies for the Hydra mainnet.
pub struct Mainnet;

impl Network for Mainnet {
    fn p2pkh_addr(&self) -> &'static [u8; 1] {
        b"\x29" // 41
    }
    fn p2sh_addr(&self) -> &'static [u8; 1] {
        unimplemented!()
    }
    fn wif(&self) -> &'static [u8; 1] {
        b"\x64" // 100
    }
    fn bip32_xprv(&self) -> &'static [u8; 4] {
        b"\x46\x08\x95\x20" // TODO
    }
    fn bip32_xpub(&self) -> &'static [u8; 4] {
        b"\x46\x09\x06\x00" // TODO
    }
    fn message_prefix(&self) -> &'static str {
        // TODO usually there is a binary length prefix, but so many btc forks screwed that
        // up (including IoP) that now many include it as part of this string. Wigy could
        // not find out whether ARK has a length prefix here and if yes, what is that.
        "HYD message:\n"
    }
    fn slip44(&self) -> i32 {
        0x485944 // 4741444
    }
}

/// Strategies for the Hydra test network (called devnet in ARK terminology).
pub struct Testnet;

impl Network for Testnet {
    fn p2pkh_addr(&self) -> &'static [u8; 1] {
        b"\x1e"
    }
    fn p2sh_addr(&self) -> &'static [u8; 1] {
        unimplemented!()
    }
    fn wif(&self) -> &'static [u8; 1] {
        b"\xaa"
    }
    fn bip32_xprv(&self) -> &'static [u8; 4] {
        b"\x46\x08\x95\x20" // TODO
    }
    fn bip32_xpub(&self) -> &'static [u8; 4] {
        b"\x46\x09\x06\x00" // TODO
    }
    fn message_prefix(&self) -> &'static str {
        "tHYD message:\n"
    }
    fn slip44(&self) -> i32 {
        0x485944 // 4741444
    }
}
