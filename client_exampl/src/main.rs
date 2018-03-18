extern crate mercury_sdk;
extern crate mercury_common;

extern crate multihash;
extern crate multiaddr;

use std::rc::Rc;

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
    pubkey : PublicKey,
    privkey : Vec<u8>,
}

impl Signo{
    fn new( whatever : &str)->Self{
        Signo{
            pubkey : PublicKey::new_from_vec(generate_hash_from_vec(generate_hash(whatever))),
            privkey : generate_hash(whatever),
        }
    }
}

impl Signer for Signo{
    fn pub_key(&self) -> &PublicKey{
        &self.pubkey
    }
    fn sign(&self, data: Vec<u8>) -> Signature{
        let mut sig = String::new();
        sig.push_str( std::str::from_utf8(&data).unwrap() );
        sig.push_str( std::str::from_utf8(&self.privkey).unwrap() );
        Signature::new( &sig )
    }
}

fn make_own_persona_profile(){
    
}

fn make_home_profile(){
    
}

fn main() {
    println!("Generating Home Addresses");
    let homeaddr1 = "/ip4/127.0.0.1/udp/1234".to_multiaddr().unwrap();
    let homeaddr2 = "/ip4/127.0.0.1/udp/2345".to_multiaddr().unwrap();
    
    println!("Generating Home Hashes");
    let home1 = generate_hash("home1");
    let home2 = generate_hash("home2");
    
    println!("Generating Profile Hashes");
    let id1 = generate_hash("prof1");
    let id2 = generate_hash("prof2");
    
    println!("Generating Signos");
    let sign1 = Signo::new("1");
    let sign2 = Signo::new("2");
    
    println!("Generating Home Profiles");
    let homeprof1 : Profile = ::Profile::new(
        &ProfileId::new_from_vec(home1), 
        &PublicKey::new("home1publickey"),
        &[ProfileFacet::Home( HomeFacet::new(&[homeaddr1]) )] 
    );
    let homeprof1 : Profile = Profile::new(
        &ProfileId::new_from_vec(home2), 
        &PublicKey::new("home2publickey"),
        &[ProfileFacet::Home( HomeFacet::new(&[homeaddr2]))] 
    );
    
    println!("Generating Persona Profiles");
    let prof1 : ::Profile = ::Profile::new(
        &ProfileId::new_from_vec(id1), 
        &PublicKey::new("profile1publickey"),
        &[ProfileFacet::Persona( PersonaFacet::new( &vec![] ) )] 
    );
    let prof2 : ::Profile = ::Profile::new(
        &ProfileId::new_from_vec(id2), 
        &PublicKey::new("profile2publickey"),
        &[ProfileFacet::Persona( PersonaFacet::new( &vec![] ) )] 
    );
    
    println!("Generating Own Datas");
    let owndata1 = OwnProfileData::new(&prof1, &[]);
    let owndata2 = OwnProfileData::new(&prof2, &[]);

    println!("Generating Own Profiles");
    let ownprof1 = OwnProfile{ data : owndata1, signer : Rc::new(sign1)};
    let ownprof2 = OwnProfile{ data : owndata2, signer : Rc::new(sign2)};
    
    
}

// impl ::PublicKey{
//     fn new( key : &str)->Self{
//         ::PublicKey(key.as_bytes().to_owned())
//     }
// }
// 
// impl ::ProfileId{
//     fn new( id : &str)->Self{
//         ::ProfileId(id.as_bytes().to_owned())
//     }
// }
// 
// impl ::Signature{
//     fn new( sign : &str)->Self{
//         ::Signature(sign.as_bytes().to_owned())
//     }
// }

// impl ::OwnProfile{
//     fn new( data : ::OwnProfileData, sign : ::Signer )->Self{
//         ::OwnProfile{
//             data:   data,
//             signer: Rc::new(sign),
//         }
//     }
// }

// impl PersonaFacet{
//     fn new( profids : [ProfileId])->Self{
//         let homesvec = vec![];
//         for prof in profids{
//              vec.add(prof);
//         }
//         PersonaFacet{homes : homesvec}
//     }
// }

// impl HomeFacet{
//     fn new(homeadds : [MultiAddr]){
//         let addrvec = vec![];
//         for addr in homeadds{
//             vec.add(addr)
//         }
//         HomeFacet{ addrs : addrvec }
//     }
// }
    