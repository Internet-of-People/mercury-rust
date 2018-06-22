# Mercury

Mercury is a distributed network built on a revolutionary person-to-person protocol 
and identity model aiming maximal privacy.
Our goal is to make the Internet ours again by true decentralised secure communication,
social networking and enabling peer-to-peer business and apps with no middlemen,
even on your phone.

Mercury is somewhat similar to a cellular mobile network, it provides features like
 SMS, calls, data connections, push notifications, etc, but
 - built as an overlay network on top of any existing transport(s), could also use wire or mesh
 - your own "cell tower" can join or leave the network any time
 - selecting a "provider" is the only trustful part of the system
 - uses cryptographic keys instead of phone numbers for p2p encrypted communication,
   cell/provider and applications use the same kind of identity
 - user data and calls are encrypted, you cannot be spied on or be cheated with contact identity
 - you're free to keep your identity and contacts moving to another provider or application
 - supports you having different unrelated identities (family, professional, hobby, etc) within the system
 - you can restore your identities from a "cold wallet" after lost/broken device
 - the network is extremely resilient, dies only with the last cell
 - built to support any decentralized/distributed application on top 

Mercury is a redesigned and advanced version of the IoP Profile Server and IoP Connect
which were a step in the right direction but lacked several features from this vision,
were created by developers who left the community and were hard to fix and maintain.   


## Project status

Please be aware that this project is still in a very early and experimental phase.
We opened up the source code to give a sneak peek to developers interested in either
developing Mercury itself or building distributed applications on top of it.
We'd like to have feedback to learn problems in the earliest phases,
priorities of missing features and your requirements we haven't thought of yet.

We think to have an initial functional implementation of the architecture.
We're working on building some details of server binaries (e.g. configuration)
and finalizing optimistic test cases, making sure they all pass.
There are still a lot of important components to be added,
existing ones might be changed or redesigned,
documentation is still lacking nearly everywhere.

Rough edges of the existing server and client are
 - documentation
 - proper error type structure
 - profile metadata structure and protection levels

Missing important parts are
 - Diffie-Hellman key exchange and encryption, maybe with TLS
 - HD Secret key generation with seeds (Bip32, Bip39)
 - hole punching support (Stun, Upnp, NatPmp, etc)
 - profile search
 - DHT integration (IPFS with custom IPNS, Kademlia or others)
 - simplified dApp SDK
 - language bindings
 - undelivered message persistance (in case of server reboot)


## Code structure

Directories/crates of the project are
 - `home-protocol` is where you should start looking to understand the architecture.
   It contains the basic definition of basic data structures, interfaces
   and network protocols plus some common utility code.
   Your starting point is maybe `protocol/mercury.capnp` describing a simple
   network protocol with Cap'n'Proto while `lib.rs` translates it to business logic in Rust. 
   Note that the `handshake` modul is only for testing, we'll drop it when a
   Diffie-Hellman key exchange is properly implemented.
 - `home-node` implements a server for the protocol.
 - `connect` implements a client. You can use `HomeClientCapnProto` from `protocol_capnp.rs`
   as a proxy object to a remote home server. File `lib.rs` aims to provide more convenience
   on top but we're planning to redesign it.
 - `test` contain integration tests to check if our server and client implementation
   work together as expected.
 - `storage` contains experimentation on a generic storage layer using hash-based "indexing"
   that could use IPFS, BitTorrent, StoreJ, etc as a simple plugin.
   We currently use only some interfaces like `KeyValueStore` from this crate,
   you should ignore it for now.

Copyright © 2017-2018
Libertaria Ventures LLP, UK
Decentralized Society Foundation, PA
