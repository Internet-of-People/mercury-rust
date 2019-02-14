use bip39::{Language, Mnemonic, MnemonicType, Seed};
use failure::Fallible;
use std::borrow::Borrow;

fn as_fallible_vec(mnemonic_res: Fallible<Mnemonic>, password: &str) -> Fallible<Vec<u8>> {
    mnemonic_res.map(|m| Seed::new(&m, password).as_bytes().to_owned())
}

pub(crate) fn generate_new(password: &str) -> Fallible<Vec<u8>> {
    as_fallible_vec(
        Ok(Mnemonic::new(MnemonicType::Words24, Language::English)),
        password,
    )
}

pub(crate) fn from_phrase<T: Into<String>>(phrase: T, password: &str) -> Fallible<Vec<u8>> {
    as_fallible_vec(Mnemonic::from_phrase(phrase, Language::English), password)
}
