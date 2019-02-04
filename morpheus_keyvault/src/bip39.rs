use std::borrow::Borrow;
use bip39::{Mnemonic, MnemonicType, Language, Seed};
use failure::Fallible;

fn as_fallible_vec(mnemonic_res: Fallible<Mnemonic>, password: &str) -> Fallible<Vec<u8>> {
    mnemonic_res
        .map(|m| Seed::new(&m, password).as_bytes().to_owned() )
}

pub(crate) fn generate_new(password: &str) -> Fallible<Vec<u8>> {
    as_fallible_vec(Ok(Mnemonic::new(MnemonicType::Words24, Language::English)), password)
}

pub(crate) fn from_phrase<T: Borrow<str>>(words: &[T], password: &str) -> Fallible<Vec<u8>> {
    let phrase = words.join(" ");
    as_fallible_vec(Mnemonic::from_phrase(phrase, Language::English), password)
}
