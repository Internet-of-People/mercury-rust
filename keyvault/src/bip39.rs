use bip39::{Language, Mnemonic, MnemonicType, Seed};
use failure::Fallible;

/// Re-exported error type used by `tiny-bip39` to allow downcasting `failure` error results.
pub use bip39::ErrorKind as Bip39ErrorKind;

/// #Panics
/// If words is not in {12, 15, 18, 21, 24}
pub(crate) fn generate_new_phrase(words: usize) -> String {
    let mnemonic = Mnemonic::new(MnemonicType::for_word_count(words).unwrap(), Language::English);
    mnemonic.into_phrase()
}

pub(crate) fn generate_new(password: &str) -> Vec<u8> {
    let mnemonic = Mnemonic::new(MnemonicType::Words24, Language::English);
    Seed::new(&mnemonic, password).as_bytes().to_owned()
}

pub(crate) fn from_phrase<S: AsRef<str>>(phrase: S, password: &str) -> Fallible<Vec<u8>> {
    let mnemonic_res = Mnemonic::from_phrase(phrase.as_ref(), Language::English);
    mnemonic_res.map(|m| Seed::new(&m, password).as_bytes().to_owned())
}

pub(crate) fn check_word(word: &str) -> bool {
    Language::English.wordmap().get_bits(word).is_ok()
}
