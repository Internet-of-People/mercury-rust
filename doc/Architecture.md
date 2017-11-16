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

In our terminology below, a **hashspace** is a Merkle-DAG (e.g. Ipfs) and
the **hashweb** is our derived Merkle-DAG construct that can connect them all.


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
[comment] # (- `Serializer` en/decodes raw in-memory objects to/from a storage format,
    e.g. bson or protobuf)
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

TODO describe `HashWeb`

TODO To shorten load time we plan to enable caching by implementing
a special composite storage `CachingKeyValueStore`.
It can contain and consult a chain of storages ordered by their access times,
e.g. an in-memory auto-pruning cache store as first entry.
When storing it should store the entry in all storages of the chain.
When loading it should grab the first successfully resolved entry from any store
and store it in the fastest (or all faster?) stores before returning it.   


Attributes
==========

Stored binary data is much more useful when paired with (at least partial)
semantic understanding of what that data means and how it is related
to other data entries.

This (meta)data is accessible through the `meta::Attribute` interface
that consists of a name and a strongly typed value.
Attribute values can be primitive (like number or timestamp), composite
(i.e. array or object that contain further attributes) or
a link to some other node.

The `common::Data` interface provides both the raw binary blob and an iterator
for its attributes with some convenience functions for easier attribute lookups.


Address resolution
==================

Edges of the Merkle-DAG are defined by the link-typed (i.e. node id)
attributes of the binary data stored in each node.
A link is a node identifier that points to some other entry,
having the format of a `(hashspace_id, hash)` pair where
hashspace_id determines the protocol how data can be resolved from a hash.

Full addresses have a human-readable format of
`hashspace_id/data_hash[#binary_format_id/path/to/a/link/attribute]*`.
The mandatory link (hashspace id and hash) identifies
an initial node in the graph. Link resolution starts by loading and validating
the binary data of this node.

After this mandatory start, the optional relative address
(a series of format id and attribute path pairs) of the link is evaluated.
To do so, the verified binary data is interpreted according the given format,
filling in all attributes during the process, including links to further nodes.
(The process is format-dependent thus specified separately for each format.)
If the attribute path refers to a link attribute, it is followed and
binary data of the referenced node is fetched.
While the relative address continues with another (format,path) pair,
the process is repeated.


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


