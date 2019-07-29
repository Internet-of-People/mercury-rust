use crate::secp256k1::Network;

/// Strategies for the ARK mainnet.
pub struct Mainnet;

impl Network for Mainnet {
    fn p2pkh_addr(&self) -> &'static [u8; 1] {
        b"\x17"
    }
    /// There is no BIP-0016 on ARK, so there is no such prefix either
    fn p2sh_addr(&self) -> &'static [u8; 1] {
        unimplemented!()
    }
    fn wif(&self) -> &'static [u8; 1] {
        b"\xaa"
    }
    fn bip32_xprv(&self) -> &'static [u8; 4] {
        b"\x46\x08\x95\x20"
    }
    fn bip32_xpub(&self) -> &'static [u8; 4] {
        b"\x46\x09\x06\x00"
    }
    fn message_prefix(&self) -> &'static str {
        // TODO usually there is a binary length prefix, but so many btc forks screwed that
        // up (including IoP) that now many include it as part of this string. Wigy could
        // not find out whether ARK has a length prefix here and if yes, what is that.
        "ARK message:\n"
    }
    fn slip44(&self) -> i32 {
        0x6f // 111
    }
}

/// Strategies for the ARK test network (devnet).
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
        b"\x46\x08\x95\x20"
    }
    fn bip32_xpub(&self) -> &'static [u8; 4] {
        b"\x46\x09\x06\x00"
    }
    fn message_prefix(&self) -> &'static str {
        "DARK message:\n"
    }
    fn slip44(&self) -> i32 {
        1
    }
}
