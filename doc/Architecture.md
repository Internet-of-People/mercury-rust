Summary
=======

A Merkle-DAG is a directed acyclic graph in which nodes contain binary data (blob)
and blob contents are referred to and validated by their hashes.
Identifying a node by the hash of its contained data,
hashes are also used to define graph edges.
There are many systems following this principle like
Git, Magnet/Torrent, ZeroNet, Ipfs or StoreJ.
   
This library aims to implement protocol-independent Merkle-DAG access that enables
 - a universal link format
 - links **between** different Merkle-DAG implementations


Basics
======

The cornerstone of the library is the `HashSpace` interface
that allows managing nodes of a Merkle-DAG.
We have both a synchronous and an asynchronous version to suit your needs.
It collects the following node operations:
 - `store` an object (i.e. create a node) and receive a hash that references it
 - `resolve` a hash to look up the object with this hash
 - `validate` a hash to an object to detect errors or tampering

Most `HashSpace` implementations are not very special and thus can be
separated into further responsibilities. `ModularHashSpace`
implements a `HashSpace` by delegating tasks to the following interfaces,
effectively delaying such implementation decisions and improving modularity:
 - `Serializer` en/decodes raw in-memory objects to/from a storage format,
    e.g. bson or protobuf
 - `Hasher` creates and validates hashes of serialized objects, e.g. sha2
 - `KeyValueStore` provides a (potentially distributed) HashMap. In other words
    it binds (stores and resolves) an arbitrary key (hash) to a value (object).
    E.g. it can use an in-memory map, local disk, sharded No/Sql or a DHT
 - `HashCoder` en/decodes binary data (hash) to/from a more human-friendly format,
    e.g. base64

To be maintainable, handle any hash and link formats
and be future-proof, our library uses the
[multibase](https://github.com/multiformats/rust-multibase) and 
[multihash](https://github.com/multiformats/rust-multihash)
multiformat cargos that include all relevant encoding and hash algorithms. 

TODO To shorten load time we plan to enable caching by implementing
a special composite storage `CachingKeyValueStore`.
It can contain and consult a chain of storages ordered by their access times,
e.g. an in-memory auto-pruning cache store as first entry.
When storing it should store the entry in all storages of the chain.
When loading it should grab the first successfully resolved entry from any store
and store it in the fastest (or all faster?) stores before returning it.   


Metadata
========

Stored data is much more useful and easily handled when paired with metadata
that describes the stored raw binary data and its relation to other data entries.

Metadata is accessible through the `meta::Attribute` trait that consists of
a name and a strongly typed value. Note that this can be a composite value
(i.e. array or object) that contains further attributes or a link
that points to other entries. Edges of the Merkle-DAG are defined
by these link attributes. 

A `meta::Data` entry provides an iterator for its attributes and
some convenience functions for easier attribute lookups.


Merkle-trees
============

A Merkle tree is a special form of a Merkle-DAG.
It is a directed graph with a tree structure that
starts from a single root node and has edges
from parent nodes to their immediate child nodes.
Only leaf nodes contain relevant binary data (and a calculated hash),
parent nodes just aggregate their own hash from the set of hashes of their
child nodes. This makes it easy to validate and modify the structure:
only hashes the changed branch has to be recalculated towards
the root, the rest of the tree can remains untouched.
This aggregation is used from transactions to blocks in blockchains,
from files to torrents in the torrent network or
from changed files to commit hashes in git.  

TODO We should be able to easily create local composite objects
from any number of subobjects as a Merkle-tree.
This would enable having "relative" links to fragments of the entry
and enable features like MAST-based (Merkle Abstract Syntax Tree)
smart contracts.


