# Use cases

## Create new profile

```bash
prometheus profile new
profile number 1, id: abcdef123456789
```

## List profiles

```bash
prometheus profile list
 profile number 1, id: abcdef123456789
 ...
```

## Show profile information

```bash
prometheus profile show --id=abcdef123456789
Details of profile id abcdef123456789:
 Public interests (one-way relations):
  type: follow, peer_id: feed....789
  ...
 Public contacts (mutual relations):
  type: friends, peer_id: deadbeef....321
  ...
```

## Create new subscription

```bash
prometheus interest new --my_id=abcdef123456789 --peer_id=feed....789 [--relation_type=follow]
Added interest to profile "feed....789" with type "follow"
```

## Create new contact

TODO is this needed for the MVP?

```bash
prometheus contact new --my_id=abcdef123456789 --peer_id=fedcba....789 [--relation_type=friends]
Initiated contact with specified peer. Please not that you'll have to wait for
approval of the contact by your peer. After approved, it will appear in your contact list.
```

## Publish

TODO Might not needed for MVP, everything could be public first

```bash
prometheus contact publish --relation_id=???
???
```
