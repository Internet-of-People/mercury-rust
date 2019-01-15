# Use cases


## Help

```bash
> prometheus help
Subcommands are:
  create (profile/link)
  set attribute
  remove attribute
  show profile
  list (profiles/followers)
  help
  version
```


## Version info

```bash
> prometheus version
Prometheus version 0.0.1-alpha1 built on Morpheus version 0.0.1-alpha2
```


## Status

Profile number is a Bip44 path fragment used to derive the keys for this profile.

```bash
> prometheus status
Your active profile is number 1, id: abcdef123456789
```


## Create new profile

```bash
> prometheus create profile [--set-active? or --keep-current?]
profile number 2, id: cab123...987
```


## List profiles

```bash
> prometheus list profiles
 (active) profile number 1, id: abcdef123456789
          profile number 2, id: cab123...987
```


## Show profile information

```bash
> prometheus show profile [--id=abcdef123456789]
Details of profile id abcdef123456789:
  Public attributes:
    "username" = "test",
    "account@twitter.com" = "cool-influencer",
    "account@youtube.com" = "influence-channel",
    ...
  Public links:
    id: "fff...aaa", type: follow, peer_id: feed....789
    ...

This profile belongs to your keyvault and is your currently active default one.
  Private attributes:
    "gender" = "male",
    "birthday" = "2000",
  Private links:
    type: "rebellion", peer_id: 2020..2077
```


## Set attribute

```bash
> prometheus set attribute --key=account@linkedin.com --value=tech-expert-123456 [--my_id=abcdef123456789]
Attribute "account@linkedin.com" was set to value "tech-expert-123456" for [active/specified] profile.
```


## Clear attribute

```bash
> prometheus clear attribute --key=account@linkedin.com  [--my_id=abcdef123456789]
Attribute "account@linkedin.com" was cleared from [active/specified] profile.
```


## Create new link

```bash
> prometheus create link --peer_id=feed....789 [--my_id=abcdef123456789] [--relation_type=follow]
Created link "aaa...fff" to "feed....789" with type "follow" for [active/specified] profile.
```


## Remove link

```bash
> prometheus remove link --link_id=aaa...fff [--my_id=abcdef123456789]
Removed link "aaa...fff" from [active/specified] profile.
```


## List public followers

TODO consider renaming "follower", based on what it means:
"links pointing to me"

```bash
> prometheus list followers [--my_id=abcdef123456789]
Profiles publicly following [active/specified] profiles are:
  Profile id: baba...666
    Attributes:
      "account@twitter.com" = "bestfan",
      ...
  Profile id: beef...42
    Attributes:
      "account@linkedin.com" = "btchodler",
      ...
```


## Publish???

TODO Might not needed for MVP, everything could be public first

```bash
> prometheus contact publish --relation_id=???
???
```
