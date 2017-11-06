Abstract
========

A Merkle-DAG is an acyclic graph in which nodes contain binary data (blob)
and blob contents are referred to and validated by their hashes.
Identifying a node by the hash of its contained data,
hashes are also used to define graph edges.
There are many systems following this principle like
Git, ZeroNet, Ipfs or StoreJ.
   
This library aims to implement universal Merkle-DAG access that enables
 - a hashgraph-agnostic link format
 - links **between** different Merkle-DAG implementations

Basics
======

To handle any hash and link formats, our library is built on
[multibase](https://github.com/multiformats/rust-multibase) and 
[multihash](https://github.com/multiformats/rust-multihash) libraries
that include all major base-encoding and hash algorithms.

