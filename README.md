# Mercury

Mercury is a distributed network built on a revolutionary person-to-person protocol 
and identity model aiming maximal privacy.
Our goal is to make the Internet ours again by true decentralised secure communication,
social networking and enabling peer-to-peer business and apps with no middlemen,
even on your phone.

## Why?

The internet was designed to provide open distributed peer to peer communication,
but your phone and PC don't have that anymore, only servers in data centers.
You're closed behind ISPs and home routers (e.g. NAT) so you need intermediaries to communicate.
Those intermediaries tie you by heavy vendor lockin: you can't change service provider
(consider email, storage, etc) without sacrificing your old identity and data.
Even worse, the biggest of them make a living from taxing your payments,
constantly spying on you for selling your data and usually hinder or censor you
for their advantage or political orders, already much further than just targeted ads. 
Just as an appetizer, some Asian countries require a mandatory digital identifier
for every payment you make and monitor all of your online activity.
Using this collected data, they plan to automatically punish or reward their citizens
based on arbitrary loyalty measures, essentially building the Thought Police.

Mercury aims to bring balance by protecting you from all of this.
Your identity is a cryptographic key owned by you and noone else.
Data storage and communication is organized around this identity which you can keep
even changing service provider or applications.
Your data is encrypted until you decide to share it with a specific peer or publicize it.
Furthermore, you can have several unconnected identities, e.g. for work, family and hobby.
The network is truly distributed and built on encrypted peer to peer communication
so you're safe. You can add your own node to the network under your control and trust.

In the end you can get rid of intermediaries or middleman and directly connect persons,
business with clients or even machines, returning from oligopoly to true competition.


### Comparison

Mercury is somewhat similar to a cellular mobile network, it provides features similar to
SMS, calls, data connections, push notifications, etc, but
 - built as an overlay network on top of any transport layer
   (currently Tcp but could use Tor, I2P, mesh, etc)
 - your own "cell tower" can join or leave the network any time
 - selecting a "provider" is the only trustful part of the system
 - uses cryptographic keys instead of phone numbers for p2p encrypted communication,
   cell/provider and applications use the same kind of identity
 - user data and calls are encrypted, you cannot be spied on or be cheated with contact identity
 - you're free to keep your identity and contacts moving to another provider or application
 - supports you having different unrelated identities (family, professional, dating, etc) within the system
 - you can restore your identities from a "cold wallet" after lost/broken device
 - the network is extremely resilient, dies only with the last cell
 - built to support any kind of decentralized/distributed application on top 


## Project status

Mercury is a redesigned and advanced version of the IoP Profile Server and IoP Connect
which were a step in the right direction but lacked several features from our vision,
were created by developers who left the community and were hard to fix and maintain.   

Please be aware that this project is still in a very early and experimental phase.
We opened up the source code to give a sneak peek to developers interested in either
developing Mercury itself or building distributed applications on top of it.
We'd like to have feedback to learn problems in the earliest phases,
priorities of missing features and your requirements we haven't thought of yet.

We think to have an initial functional implementation of the architecture.
We're currently merging in the KeyVault and Open Social Graph codebase
while drafting our SDK API for distributed applications.
There are still a lot of important components to be added,
existing ones might be changed or redesigned,
documentation is still lacking nearly everywhere.

Experimental features present:
 - Storage plugin interfaces (considering potentially distributed storages) 
 - Protocol for communication between Home node and clients
 - Home node binaries
 - Client library

Merging in:
 - KeyVault: HD Secret key generation with seeds (Bip32, Bip39, etc)
 - Open Social Graph features

Rough edges of the existing server and client are
 - documentation
 - profile metadata structure and protection levels

Missing important parts are
 - dApp SDK, including
   - native GUI plugins interfaces for profile, home and contact management
   - hiding all possible tech details to be convenient
 - Diffie-Hellman key exchange and encryption, maybe with TLS
 - hole punching support (Stun, Upnp, NatPmp, etc)
 - DHT integration (IPFS with custom IPNS, Kademlia or others)
 - profile search on distributed storage
 - language bindings
 - undelivered message persistance (e.g. missed calls)


## Code structure

Directories/crates of the project are
 - `home-protocol` is where you should start looking to understand the architecture.
   It contains the basic definition of basic data structures, interfaces
   and network protocols plus some common utility code.
   Your starting point is maybe `protocol/mercury.capnp` describing a simple
   network protocol with Cap'n'Proto while `mercury-capnp/mod.rs` contains
   client and server implementations for Rust. 
 - `home-node` implements a server for the protocol.
 - `connect` implements a client library.
 - `test` contain integration tests to check if our server and client implementation
   work together as expected.
 - `storage` contains experimentation on a generic storage layer using hash-based "indexing"
   that could use IPFS, BitTorrent, StoreJ, etc as a simple plugin.

Copyright © 2017-2019  
Libertaria Ventures LLP, UK  
Decentralized Society Foundation, PA
