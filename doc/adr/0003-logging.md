# 0003. Logging in mercury stack

Date: 2018-07-19

## Status

RFC

## Context

It'd be convenient to have a well-defined logging strategy for our applications. It makes controlling logging easier, and provides post-mortem debugging with useful data.

## Decision

We use the log4rs (https://github.com/sfackler/log4rs) crate for logging purposes. 

### Loglevels ###

We choose loglevels for messages based on the following criteria:
- error!(): When an unexpected condition occurs which likely results in application termination. Usually coupled with an Error struct.
- warn!(): When some error happens, which can be likely recovered. Usually coupled with an Error struct.
- info!(): Status message about application level progress, important state changes.
- debug!(): Additional program state info that can be used for debugging.
- trace!(): High frequency or very detailed messages, that helps precisely tracking program execution/state.

### Controlling loglevels from the command line or config file ###

The default loglevel is "info", this means that info!(), warn!() and error!() level messages are shown. Loglevel can changed by -v/--verbose or -s/--silent command line options.
- -s -s -s => loglevel == none
- -s -s => loglevel == err
- -s => loglevel == warn
- -v => loglevel == debug
- -v -v => loglevel == trace

[Optional proposal]

Loglevel can be specified with -l/--loglevel <LOGLEVEL> command line option (where loglevel can be trace, debug, info, warn, error, none)

### Controlling log output ###

By default log output shall be sent to the console and to a file defaulting to a well defined place (e.g.: /var/log/<appname>.log). Two options can be used to specify alternative output:
- --syslog: send log output to syslog
- --logfile <LOGFILE>: send log output to a file

Log rotation should be handled by an external package (logrotate or similiar) if possible.

### Printing Error structs ###

It's very important for post-mortem debugging that error messages are logged properly. Usually an error consists of two parts:
- message (what went wrong?)
- error code (how the system responded?), this is usually having a numeric and a textual description

Example:

In case of a network issue we might face connection issues. We should report it as:

"failed to connect to homeserver at xxx.xxx.xxx:xx (111: Connection refused)"

Error structs can be stacked. We have to take care to log internal errors as well to see the full error stack.

### Log messages ###

Log messages should be compact readable and informative. It's hard to give exact rules, but the following advices must be followed.

- avoid irrelevant messages (e.g. instead of the message "program started", one can print also important arguments, program version, ... too)
- for error messages always provide enough context for later analysis (important state variables)
- avoid dumping long binary data, public keys, memory garbage. If binary content should be tracked, consider logging some hash of the content.
- messages which are issued from tight loops with big frequency, should always happen on the trace!() loglevel to avoid flooding the log output
- avoid writing multiple sentences (instead use single sentence without capitalization), that helps log processing with scripts
- types that are usually written into logs shall implement the Display trait, to provide compact and informative output

## Consequences

By following these guidelines we'd have consistent, readable logs that help debugging and operation.