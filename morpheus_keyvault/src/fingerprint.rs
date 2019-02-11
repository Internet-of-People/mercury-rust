use blake2::{
    digest::{Input, VariableOutput},
    VarBlake2b,
};

use super::mbase::mbase58_encode;

pub fn fingerprint<B: AsRef<[u8]>>(data: B) -> String {
    let mut hasher = VarBlake2b::new(16).unwrap();
    hasher.input(data);
    let hash = hasher.vec_result();
    let mut output = mbase58_encode(&hash);
    output.insert(0, 'I');
    output
}

#[cfg(test)]
mod tests {
    use super::fingerprint;

    #[test]
    fn test_fingerprint() {
        let data = [0x00u8, 0x01, 0x0f, 0x10, 0xfe, 0xff];
        let output = fingerprint(&data);

        assert_eq!(&output, "IzUe6j1ty4HMjUyT6kLoAU2z");
    }

    #[test]
    fn test_empty_fingerprint() {
        let data: [u8; 0] = Default::default();
        let output = fingerprint(&data);

        assert_eq!(&output, "IzS4BtphVU3QBtZNNc8aXM27");
    }

    #[test]
    fn test_big_fingerprint() {
        let mut data = Vec::with_capacity(16_777_216);
        unsafe { data.set_len(16_777_216) }
        let mut next = 0u8;
        for item in data.iter_mut() {
            *item = next;
            next = next.wrapping_add(1);
        }
        let output = fingerprint(&data);

        assert_eq!(&output, "IzEjhPJoCxi8cv33uzGDMm1d");
    }
}
