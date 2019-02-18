use bip39::{Language, Mnemonic, MnemonicType, Seed};
use failure::Fallible;

pub(crate) fn generate_new_phrase(words: usize) -> String {
    let mnemonic = Mnemonic::new(
        MnemonicType::for_word_count(words).unwrap(),
        Language::English,
    );
    mnemonic.into_phrase()
}

pub(crate) fn generate_new(password: &str) -> Vec<u8> {
    let mnemonic = Mnemonic::new(MnemonicType::Words24, Language::English);
    Seed::new(&mnemonic, password).as_bytes().to_owned()
}

pub(crate) fn from_phrase<T: Into<String>>(phrase: T, password: &str) -> Fallible<Vec<u8>> {
    let mnemonic_res = Mnemonic::from_phrase(phrase, Language::English);
    mnemonic_res.map(|m| Seed::new(&m, password).as_bytes().to_owned())
}
