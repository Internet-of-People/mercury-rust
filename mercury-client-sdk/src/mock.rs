use super::*;
use mercury_common::*;

use multihash::{encode, Hash};
use multiaddr::{Multiaddr, ToMultiaddr};



fn generate_hash( base : &str) -> Vec<u8> {
    encode(Hash::SHA2256, base.as_bytes()).unwrap()
}

fn generate_hash_from_vec( base : Vec<u8>) -> Vec<u8> {
    encode(Hash::SHA2256, &base).unwrap()
}

struct Signo{
    prof_id : ProfileId,
    pubkey : PublicKey,
    privkey : Vec<u8>,
}

impl Signo{
    fn new( whatever : &str)->Self{
        Signo{
            prof_id : ProfileId("MockSigner".as_bytes().to_owned()),
            pubkey : PublicKey(generate_hash_from_vec(generate_hash(whatever))),
            privkey : generate_hash(whatever),
        }
    }
}

impl Signer for Signo{
    fn prof_id(&self) -> &ProfileId{
        &self.prof_id
    }
    fn pub_key(&self) -> &PublicKey{
        &self.pubkey
    }
    fn sign(&self, data: Vec<u8>) -> Signature{
        let mut sig = String::new();
        sig.push_str( std::str::from_utf8(&data).unwrap() );
        sig.push_str( std::str::from_utf8(&self.privkey).unwrap() );
        Signature( sig.into_bytes() )
    }
}

struct DummyHome {
    signer : Signo,
    ping_reply: String,
}

impl DummyHome {
    fn new(ping_reply: &str) -> DummyHome {
        DummyHome {
            signer: Signo::new("Mockarony"),
            ping_reply: String::from(ping_reply),
        }
    }
}

impl PeerContext for DummyHome {
    fn my_signer(&self) -> &Signer {
        &self.signer
    }
    fn peer(&self) -> Option<Profile>{
        None
    }
    fn peer_pubkey(&self) -> Option<PublicKey>{
        None
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn instantiate_signo(){
        let _signo = Signo();
        assert_eq!(3, 4);
    }
}
