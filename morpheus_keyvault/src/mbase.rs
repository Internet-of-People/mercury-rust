//! A small lightweight replacement for the multibase crate for base58btc encoding/decoding binary data
use failure::{ensure, Fallible};

const ALPHA58BTC: &[u8] = b"123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz";

/// Apply base58btc encoding on data
pub fn mbase58_encode<B: AsRef<[u8]>>(data: B) -> String {
    let mut output = base_x::encode(ALPHA58BTC, data.as_ref());
    output.insert(0, 'z');
    output
}

/// Decode mulibase encoded string into binary data. Only base58btc 'z' is supported at the moment
pub fn mbase_decode(input: &str) -> Fallible<Vec<u8>> {
    let mut chars = input.chars();
    let first = chars.next();
    ensure!(first.is_some(), "Missing multibase prefix");
    ensure!(
        first.unwrap() == 'z',
        "Can only decode base58 with a prefix 'z'"
    );
    let data = base_x::decode(ALPHA58BTC, chars.as_str())?;
    Ok(data)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encode() {
        let data = [0x00u8, 0x01, 0x0f, 0x10, 0xfe, 0xff];
        let output = mbase58_encode(&data);

        assert_eq!(output, "z17vsMUr");
    }

    // https://github.com/multiformats/rust-multibase/issues/5
    #[test]
    fn decode() {
        let input = "z17vsMUr";
        let data = mbase_decode(input).unwrap();

        assert_eq!(hex::encode(data), "00010f10feff");
    }

    #[test]
    fn decode_empty() {
        let input = "z";
        let data = mbase_decode(input).unwrap();

        assert!(data.is_empty());
    }

    #[test]
    fn decode_failure() {
        let input = "z17vsMUl"; // Invalid character 'l'
        let err = mbase_decode(input).unwrap_err();
        let description = err.compat().to_string();
        assert!(description.contains("Failed to decode the given data"));
    }
}
