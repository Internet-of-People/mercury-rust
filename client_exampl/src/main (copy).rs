extern crate mercury_sdk;
extern crate mercury_common;

extern crate multihash;
extern crate multiaddr;

use std::rc::Rc;

use multihash::{encode, Hash};
use multiaddr::{Multiaddr, ToMultiaddr};

fn generate_profile_id( base : &str) -> Vec<u8> {
    encode(Hash::SHA2256, base.as_bytes()).unwrap()
}

// struct Signo{
//     pubkey : mercury_common::PublicKey,
//     PrivateKey : Vec<u8>,
// }
// 
// impl Signo{
//     fn new()->Self{
// 
//     }
// }
// 
// impl mercury_common::Signer for Signo{
//     fn pub_key(&self) -> &mercury_common::PublicKey{
//         &self.pubkey
//     }
//     fn sign(&self, data: Vec<u8>) -> mercury_common::Signature{
//         self.signature.clone()
//     }
// }

// impl mercury_common::PublicKey{
//     fn new( key : &str)->Self{
//         mercury_common::PublicKey(key.as_bytes().to_owned())
//     }
// }
// 
// impl mercury_common::ProfileId{
//     fn new( id : &str)->Self{
//         mercury_common::ProfileId(id.as_bytes().to_owned())
//     }
// }
// 
// impl mercury_common::Signature{
//     fn new( sign : &str)->Self{
//         mercury_common::Signature(sign.as_bytes().to_owned())
//     }
// }

// impl mercury_common::OwnProfile{
//     fn new( data : mercury_common::OwnProfileData, sign : mercury_common::Signer )->Self{
//         mercury_common::OwnProfile{
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
    
fn main() {
    let homeaddr1 = "/ip4/127.0.0.1/udp/1234".to_multiaddr().unwrap();
    let homeaddr2 = "/ip4/127.0.0.1/udp/2345".to_multiaddr().unwrap();
    let home1 = generate_profile_id("home1");
    let home2 = generate_profile_id("home2");
    
    let id1 = generate_profile_id("prof1");
    let id2 = generate_profile_id("prof2");
    

    let homeprof1 : mercury_common::Profile = mercury_common::Profile::new(
        &mercury_common::ProfileId(home1), 
        &mercury_common::PublicKey("home1publickey".as_bytes().to_owned()),
        &[mercury_common::ProfileFacet::Home( mercury_common::HomeFacet{ addrs : vec![ homeaddr1 ] } )] 
    );
    let homeprof1 : mercury_common::Profile = mercury_common::Profile::new(
        &mercury_common::ProfileId(home2), 
        &mercury_common::PublicKey("home2publickey".as_bytes().to_owned()),
        &[mercury_common::ProfileFacet::Home( mercury_common::HomeFacet{ addrs : vec![ homeaddr2 ] } )] 
    );
    

    let prof1 : mercury_common::Profile = mercury_common::Profile::new(
        &mercury_common::ProfileId(id1), 
        &mercury_common::PublicKey("profile1publickey".as_bytes().to_owned()),
        &[mercury_common::ProfileFacet::Persona( mercury_common::PersonaFacet{ homes : vec![] } )] 
    );
    let prof2 : mercury_common::Profile = mercury_common::Profile::new(
        &mercury_common::ProfileId(id2), 
        &mercury_common::PublicKey("profile2publickey".as_bytes().to_owned()),
        &[mercury_common::ProfileFacet::Persona( mercury_common::PersonaFacet{ homes : vec![] } )] 
    );
    let owndata1 = mercury_common::OwnProfileData::new(&prof1, &[]);
    let owndata2 = mercury_common::OwnProfileData::new(&prof2, &[]);
    
    let ownprof1 = mercury_common::OwnProfile::new(owndata1, sign1);
    let ownprof2 = mercury_common::OwnProfile::new(owndata2, sign2);
}
