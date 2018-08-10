# 0004. Error handling in the Mercury stack

Date: 2018-08-08

## Status

RFC

## Context

Since some of our applications are expected to operate in an error resistant way, it's important to lay out how we detect and manage unexpected situations. The prinicples described in this article are just partially Rust-specific, some of them applies to the design or testing activity space.

## Decision

Principles described below should be followed by every contributor. Existing code should be refactored ASAP.

## Guidelines

### Design for error resilience

Our applications are operating in an unstable and possibly hostile environment, so we have to plan for error conditions to happen quite often. The following guidelines have to be taken into account:

- Design communication protocols to detect hostile/badly behaving peers early, and disconnect immediately.
- In case of connection issues, make reconnection a polite process (don't try to reconnect in a tight loop). Consider to apply exponential reconnection strategies (eventually wrap that one into a component). https://en.wikipedia.org/wiki/Exponential_backoff
- For network protocols be relaxed on what we accept and be strict about what we send. That would enable error-free operation across multiple versions of different components.
- Impose strict limits on buffering (across all levels of the protocol stack) to prevent erroneous/hostile peers to take down our applications via overflow.
- For every operation take into account how we detect and report errors that are related to that specific operation.
- Try to scale protocols well by minimizing resources assigned to requests/connections.
- If possible, apply back-pressure to minimize queueing.
- Protocol designs and implementations has to have a semi-formally defined format beyond source code (e.g. FSM: https://en.wikipedia.org/wiki/Communicating_finite-state_machine).
- Don't make assumptions on unreliable channels, plan for implementing timeouts for unreliable operations to avoid deadlocks.

### Implement error handling (Rust specific)

- Use standard error handling primitives for error handling (std::Error, std::Result, ...). In case of low level codes, wrap up error codes into higher level constructs ASAP (in the best case naked error codes should not even show up even on private interfaces).
- Panics should always signal critical errors that needs attention from us. A panic in production always mean that either the deployment is broken, or we have a bug in our hands. That implies that panics need to be recorded properly (call stack, log, eventual error report from users, ...). That also means that naked .unwrap() calls are almost always a bad thing.
- Never throw away errors, even if they are not affecting normal operation. It still worth at least logging them. This implies that some compiler warnings should be eventually treated as errors that need to be fixed.
- In layered architectures it's important to preserve low-level error codes while attaching higher level error as the errors are travelling through the stack. For that purpose we need to consider the usage of crates that help to implement structured error handling. One such example is https://github.com/rust-lang-nursery/error-chain (credit goes to wigy).
- During the development of tests implement negative tests. Also be careful on checking that after an error condition is recoverable, so the code can return to normal operation after an error.
- Try to create stress tests, some of errors or protocol deficiences only appear under heavy load. This is even more important for code which is aimed at the relatively resource constrained Titania box.
- Post-mortem analysis requires that as much information is available as possible. That means that logs and backtrace has to be persisted for further study. For that purpose we might want to consider using (https://docs.rs/human-panic/1.0.0/human_panic/ credit goes to Bartmoss).

## Concrete error handling example with error chains in a layered design ##

Lower layer:

```
error_chain! {
    errors {
        LowLevelConnection(msg: String) {
            description("low level connection failure")
            display("low level connection failure: {}", msg)
        }
    }
}
 
pub fn do_something_low_level() -> Result<()> {
    Err(::std::io::Error::new(::std::io::ErrorKind::ConnectionAborted, "failed to connect to server")).chain_err(|| ErrorKind::LowLevelConnection("connect()".to_string()))
}
```

Higher layer:

```
error_chain! {
    errors {
        HighLevelConnection(msg: String) {
            description("high level connection failure")
            display("high level connection failure: {}", msg)
        }
 
 
    }
}
 
pub fn do_something_high_level() -> Result<()> {
    ::lower_layer::do_something_low_level().chain_err(|| ErrorKind::HighLevelConnection("peer connection failed".to_string()))
}
```

Application layer:

```
#![recursion_limit = "1024"]
 
 
#[macro_use]
extern crate error_chain;
 
mod higher_layer;
mod lower_layer;
 
 
fn main() {
    if let Err(ref e) = higher_layer::do_something_high_level() {
        println!("error: {}", e);
 
        for e in e.iter().skip(1) {
            println!("caused by: {}", e);
        }
 
        // The backtrace is not always generated. Try to run this example
        // with `RUST_BACKTRACE=1`.
        if let Some(backtrace) = e.backtrace() {
            println!("backtrace: {:?}", backtrace);
        }
 
        ::std::process::exit(1);
    }
}
```

Outcome:

```
hcmbp:error-handling andrei$ cargo run
    Finished dev [unoptimized + debuginfo] target(s) in 0.03s
     Running `target/debug/error-handling`
error: high level connection failure: peer connection failed
caused by: low level connection failure: low level connection failed somehow
caused by: failed to connect to server
```