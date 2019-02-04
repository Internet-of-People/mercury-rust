use std::sync::Mutex;
use std::borrow::Borrow;
use bip39::{Mnemonic, MnemonicType, Language};
use failure::Fallible;

// Simple glue code between `error-chain` used by bip39-rs 0.5.1 and `failure` used by us.
// Fail implementations needs to be Sync and Mutex can achieve that.
#[derive(Debug, Fail)]
#[fail(display = "bip39-rs: {:#?}", e)]
pub(crate) struct Bip39Error {
    e: Mutex<bip39::Error>,
}

fn as_fallible_vec(mnemonic_res: Result<Mnemonic, bip39::Error>) -> Fallible<Vec<u8>> {
    mnemonic_res
        .map(|m| m.as_seed().as_bytes().to_owned() )
        .map_err(|e| Bip39Error { e: Mutex::from(e) }.into() )
}

pub(crate) fn generate_new(password: &str) -> Fallible<Vec<u8>> {
    as_fallible_vec(Mnemonic::new(MnemonicType::Type24Words, Language::English, password.to_owned()))
}

pub(crate) fn from_phrase<T: Borrow<str>>(words: &[T], password: &str) -> Fallible<Vec<u8>> {
    let phrase = words.join(" ");
    as_fallible_vec(Mnemonic::from_string(phrase, Language::English, password.to_owned()))
}
