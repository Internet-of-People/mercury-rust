# 0001. Choosing Rust

Date: 2017-10-17

## Status

Accepted

## Context

IoP had a Mercury/Connect SDK before this, but it was implemented in Java/C# and it was not maintainable anymore. We had to decide if we want to maintain somehow or
rewrite it from scratch.

## Decision

The Rust language was chosen to be used based on it’s almost C level speed and rusts memory safety.
The language also possesses really good bindings. Basically you can bind any code written in C into Rust.
While Rust is still in its early years, it’s growing steadily, and it also has a good, stable, and growing community.

## Consequences

As we rewrite it from scratch, we have the chance to change the architecture.