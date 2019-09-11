# Mercury

Mercury
 - is a decentralized, open and secure communication infrastructure.
 - guarantees ownership of your digital identity and data, aiming maximal user privacy.
 - has a networking model that allows secure connectivity of end user devices without central services.
 - prevents lockin to service providers or applications, allows easier migration.
 - enables open social networking without middlemen.

Note that this repository contains the infrastructure backend, consisting of
background services and command line tools. For a good user experience, you can
[use a web frontend](https://github.com/Internet-of-People/prometheus-ui)
built in node.js on top of the backend or Electron-based
[standalone application binaries](https://github.com/Internet-of-People/prometheus-electron)
bundling both the backend and the web frontend.

## Why?

The internet was designed to provide open and distributed peer to peer communication,
but your phone and PC don't have that anymore, only servers in data centers.
You're closed behind ISPs and home routers (e.g. NAT) so you need intermediaries to communicate.
Those intermediaries tie you by heavy vendor lockin: you can't change service provider
(consider email, social networks, online storage, etc) without sacrificing your old identity and data.
The biggest of them make a living from taxing all of your payments in their stores,
constantly spying on you for selling targeted ads and your data to partners and
usually hinder or censor you for political agendas or any other reasons.

Mercury aims to protect you from all of this.
Your identity is built on cryptographic keys owned by you alone. These keys are disposable,
so you can split your digital footprint into as many unrelated profiles as needed,
e.g. for work, family and hobby. 
Data storage and communication is organized around such profiles which you can keep
even changing service provider or applications.
Your data is encrypted until you decide to share a part of it with a specific peer or the general public.
The network is truly distributed and built on encrypted peer to peer communication so you're safe.
You can add your full node to the network under your own control and
use your end device as a light client of a node you trust.
In the end you can get rid of intermediaries or middleman and directly connect persons,
business with clients or even machines.


### Comparison

Mercury is somewhat similar to a cellular mobile network, it provides features similar to
SMS, calls, data connections, push notifications, etc, but
 - built as an overlay network on top of any transport layer
   (currently Tcp but could use Tor, I2P, mesh, etc)
 - your own "cell tower" node can join or leave the network any time
 - selecting a "provider/cell" is the only trustful part of the system and can be changed any time
 - uses cryptographic keys instead of phone numbers to connect peer to peer,
   nodes and applications use the same kind of identity
 - user data and calls are encrypted, you cannot be spied on or be cheated with contact identity
 - you're free to keep your identity and contacts moving to another provider or application
 - supports you having different unrelated identities (family, professional, dating, etc) within the system
 - you can restore all your profiles from a "cold HD wallet" after lost/broken device
 - the network is extremely resilient, dies only with the last cell
 - built to support any kind of decentralized/distributed application on top 

Mercury's identity, data and relations model has the same vision as
[W3C Distributed IDs](https://w3c-ccg.github.io/did-spec) and
[W3C Verifiable credentials/claims](https://w3c.github.io/vc-data-model/)
but is radically simpler without carrying excess burdens of legacy webstack support.
Mercury's storage layer is built on content-hashable network principles similarly to e.g.
[Sidetree](https://github.com/decentralized-identity/sidetree/blob/master/docs/protocol.md). 

## Project status

Mercury is a redesigned and advanced version of the IoP Profile Server and IoP Connect
which were a step in the right direction but lacked several features from our vision,
were created by developers who left the community and were hard to fix and maintain.   

Please be aware that this project is still in an early and experimental phase.
We opened up the source code to give a sneak peek to developers interested in either
developing Mercury itself or building distributed applications on top of it.
We'd like to have feedback to learn problems in the earliest phases,
priorities of missing features and your requirements we haven't thought of yet.

We think to have an initial functional implementation of the architecture.
There are still a lot of important components to be added,
existing ones might be changed or redesigned and
documentation is still lacking.

Experimental features already available:
 - "Cold HD wallet" support: restore all of your profiles and related data from a single seed phrase
 - Initial Open Social Graph features
 - Storage plugin interfaces (considering potentially distributed storages)
 - Protocol for communication between Home node and clients
 - Home node binaries
 - Client library
 - Sample dApp

Rough edges of the existing server and client are
 - documentation
 - profile metadata structure and protection levels

Missing important parts are
 - Finished dApp SDK, including
   - native GUI plugins interfaces for profile, home and contact management
   - hiding all possible tech details to be convenient
 - Diffie-Hellman key exchange, data encryption
 - Verifiable claims
 - hole punching support (Stun, Upnp, NatPmp, etc)
 - DHT integration (IPFS with custom IPNS, Kademlia-variants or others)
 - profile search on distributed storage
 - language bindings
 - undelivered message persistance (e.g. missed calls)


## Code structure

Directories/crates of the project are
 - `keyvault` provides hierarchical deterministic key generation for
   multiple different cipher suites and unified serialization of
   cryptographic components (public and secret keys, ids, signatures, etc).
 - `did` aligns our `keyvault` implementation with decentralized identities from W3C.
 - `claim` implements verifiable claims as a foundation for certificates,
   social relations and shareable user data in general
 - `prometheus` provides a backend library for handling your identities and claims
   and a daemon binary for exposing library calls to external GUIs
 - `prometheus-cli` implements a command line tool as the simplest user interface
   to this daemon 
 - `home-protocol` contains the basics for network communication, defining
   services provided by home nodes operating the network and how clients can use these services.
   File `protocol/mercury.capnp` describes a simple network protocol with Cap'n'Proto
   while `mercury-capnp/mod.rs` contains client and server implementations for Rust. 
 - `home-node` implements the server side by providing the services of the protocol to clients.
 - `connect` implements the client side of the protocol. This includes an admin API to manage your
   profiles and an dApp SDK providing common building blocks to create distributed applications.
 - `examples/TheButton` is a sample distributed application built on the dApp SDK  
 - `test` and `prometheus-test` contain integration tests between different crates.
 - `storage` contains experimentation on a generic storage layer using hash-based "indexing"
   that could use IPFS, BitTorrent, StoreJ, etc as a simple plugin.
 - `forgetfulfuse` contains experiments with a filesystem that is readable only temporarily,
   planned to be used for protecting sensitive data, e.g. your keys 

Copyright © 2017-2019, Decentralized Society Foundation, PA
