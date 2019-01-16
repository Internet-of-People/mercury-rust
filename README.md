# Use cases

## Help

TODO ???remove profile???

```
> prometheus help
Subcommands are:
  status
  list (profiles/followers)
  show profile
  activate profile
  create (profile/link)
  remove link
  set attribute
  clear attribute
  help
  version
```


## Version info

```
> prometheus version
Prometheus version 0.0.1-alpha1 built on Morpheus version 0.0.1-alpha2
```


## Status

Profile number is a Bip32 path fragment used to derive the keys for this profile.

```
> prometheus status
Your active profile is number 1, id: abcdef123456789
```


## Create new profile

```
> prometheus create profile [--set-active? or --keep-current?]
profile number 2, id: cab123...987
```


## List profiles

```
> prometheus list profiles
 (active) profile number 1, id: abcdef123456789
          profile number 2, id: cab123...987
```


## Activate profile

```
> prometheus activate profile [--number=1] or [--id=abcdef123456789]
Profile number 1, id: abcdef123456789 is now your active default profile.
```


## Remove profile

TODO what does `remove profile` mean? Is it possible at all? Is it needed?


## Show profile information

```
> prometheus show profile [--id=abcdef123456789]
Details of profile id abcdef123456789:
  Public attributes:
    "username" = "test",
    "com.twitter.account" = "cool-influencer",
    "com.youtube.account" = "influence-channel",
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

```
> prometheus set attribute --key=com.linkedin.account --value=tech-expert-123456 [--my_profile_id=abcdef123456789]
Attribute "com.linkedin.account" was set to value "tech-expert-123456" for [active/specified] profile.
```


## Clear attribute

```
> prometheus clear attribute --key=com.linkedin.account  [--my_profile_id=abcdef123456789]
Attribute "com.linkedin.account" was cleared from [active/specified] profile.
```


## Create new link

```
> prometheus create link --peer_id=feed....789 [--my_profile_id=abcdef123456789] [--relation_type=follow]
Created link "aaa...fff" to "feed....789" with type "follow" for [active/specified] profile.
```


## Remove link

```
> prometheus remove link --link_id=aaa...fff [--my_profile_id=abcdef123456789]
Removed link "aaa...fff" from [active/specified] profile.
```


## List public followers

This command will list the incoming links pointing to your profile (i.e. your followers).
Note that outgoing links (your subscriptions) are shown by command `show profile`.

```
> prometheus list followers [--my_profile_id=abcdef123456789]
Profiles publicly following [active/specified] profile are:
  Profile id: baba...666
    Attributes:
      "com.twitter.account" = "bestfan",
      ...
  Profile id: beef...42
    Attributes:
      "com.linkedin.account" = "btchodler",
      ...
```
