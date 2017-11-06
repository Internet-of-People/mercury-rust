Summary
=======

A Merkle-DAG is a directed acyclic graph in which nodes contain binary data (blob)
and blob contents are referred to and validated by their hashes.
Identifying a node by the hash of its contained data,
hashes are also used to define graph edges.
There are many systems following this principle like
Git, Magnet/Torrent, ZeroNet, Ipfs or StoreJ.
   
This library aims to implement universal Merkle-DAG access that enables
 - a hashgraph-agnostic link format
 - links **between** different Merkle-DAG implementations


Basics
======

The cornerstone of the library is the `HashSpace` interface
that allows managing nodes of a Merkle-DAG.
We have both a synchronous and an asynchronous version to suit your needs.
It collects the following node operations:
 - `store` an object (i.e. create a node) and receive a hash that references it
 - `resolve` a hash to fetch the object from which it was hashed
 - `validate` a hash to an object to detect errors or tampering

Most `HashSpace` implementations are not very special and thus can be
separated into further responsibilities. `CompositeHashSpace`
implements a `HashSpace` by delegating tasks to the following interfaces,
effectively delaying such implementation decisions and improving modularity:
 - `Serializer` en/decodes raw in-memory objects to/from a storage format, e.g. bson or protobuf
 - `Hasher` creates and validate hashes of serialized objects, e.g. sha2
 - `KeyValueStore` provides a potentially distributed HashMap,
    i.e. binds (stores and resolves) an arbitrary key (hash) with a value (object),
    e.g. in-memory, to a local DB or to a DHT
 - `StringCoder` en/decodes binary data (hash) to/from a more human-friendly format, e.g. base64

To be maintainable, handle any hash and link formats
and be future-proof, our library uses the
[multibase](https://github.com/multiformats/rust-multibase) and 
[multihash](https://github.com/multiformats/rust-multihash)
multiformat cargos that include all relevant encoding and hash algorithms.


Composite objects
=================

TODO We should be able to create composite objects from an arbitrary number of subobjects,
i.e. enable easy creation of local Merkle-trees like a Torrent file. How to do it?
Rust traits with `Box` makes them hard to serialize and hash. 


Metadata
========

Stored binary data is much more useful and easily handled when paired with metadata
that describes the stored raw data and also shows other entries it is related to.

TODO how to represent metadata? Rust traits with `Box` make it hard to serialize and hash.
