// NOTE based on the 'names' crate but made deterministic https://github.com/fnichol/names
mod adjectives;
mod nouns;

use adjectives::ADJECTIVES;
use nouns::NOUNS;

pub struct DeterministicNameGenerator<'a> {
    adjectives: &'a [&'a str],
    nouns: &'a [&'a str],
}

impl<'a> DeterministicNameGenerator<'a> {
    pub fn new(adjectives: &'a [&'a str], nouns: &'a [&'a str]) -> Self {
        DeterministicNameGenerator { adjectives, nouns }
    }

    fn random_word(&'a self, data: &[u8], words: &[&'a str]) -> &str {
        let mut seed = [0u8; 32];
        let seed_len = std::cmp::min(data.len(), seed.len());
        seed[..seed_len].clone_from_slice(&data[..seed_len]);

        use rand::{distributions::Uniform, Rng, SeedableRng};
        let mut rng = rand_chacha::ChaChaRng::from_seed(seed);
        let idx = rng.sample(Uniform::new(0, words.len()));
        words[idx]
    }

    fn adjective(&self, data: &[u8]) -> &str {
        self.random_word(data, self.adjectives)
    }
    fn noun(&self, data: &[u8]) -> &str {
        self.random_word(data, self.nouns)
    }

    pub fn name(&self, data: &[u8]) -> String {
        format!("{} {}", self.adjective(data), self.noun(data))
    }
}

impl<'a> Default for DeterministicNameGenerator<'a> {
    fn default() -> Self {
        DeterministicNameGenerator::new(ADJECTIVES, NOUNS)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deterministic_names() {
        let did = [0u8; 32];
        let name = DeterministicNameGenerator::default().name(&did);
        assert_eq!(name, "Neoteric Nurture");

        let did = [41u8; 64];
        let name = DeterministicNameGenerator::default().name(&did);
        assert_eq!(name, "Hortative Heir");

        let did = [42u8; 32];
        let name = DeterministicNameGenerator::default().name(&did);
        assert_eq!(name, "Beneficial Care");

        let did = [255u8; 16];
        let name = DeterministicNameGenerator::default().name(&did);
        assert_eq!(name, "Venust Warden");
    }
}
