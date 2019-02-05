use super::Seed;

const fn as_bytes(bits: usize) -> usize {
    bits / 8
}

#[test]
fn seed_from_bytes_accepts_512_bits() {
    let bytes = [0u8; as_bytes(Seed::BITS)]; // 512 bits
    let seed_res = Seed::from_bytes(&bytes);
    assert!(seed_res.is_ok());
}

#[test]
fn seed_from_bytes_rejects_not_512_bits() {
    let bytes = [0u8; 32]; // 256 bits
    let seed_res = Seed::from_bytes(&bytes);
    assert!(seed_res.unwrap_err().to_string().contains("-bit seed"));
}
