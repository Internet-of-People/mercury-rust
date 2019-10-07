# HTTP API for `prometheusd`

Daemon `prometheusd` accepts incoming REST-like client connections on `127.0.0.1:8080` by default.
This can be overridden using configuration option `--listen IP:PORT`.

## Table of contents <!-- omit in toc -->

- [Authentication and/or authorization](#Authentication-andor-authorization)
- [BIP39 seed phrases](#BIP39-seed-phrases)
  - [Generate seed phrase](#Generate-seed-phrase)
  - [Validate seed word](#Validate-seed-word)
  - [Validate seed phrase](#Validate-seed-phrase)
- [Vault initialization](#Vault-initialization)
- [Profile management](#Profile-management)
  - [List all profiles](#List-all-profiles)
  - [Load a single profile](#Load-a-single-profile)
  - [Create new profile](#Create-new-profile)
  - [Rename profile](#Rename-profile)
  - [Change avatar picture](#Change-avatar-picture)
- [Claims](#Claims)
- [Claim schemas](#Claim-schemas)
  - [List all claim schemas](#List-all-claim-schemas)

## Authentication and/or authorization

TODO

## BIP39 seed phrases

### Generate seed phrase

Generate random entropy for a new keyvault, i.e. a new BIP39 "cold wallet" for the user.  

Request:

- Endpoint: POST `/bip39`
- Parameters: -
- Headers: -
- Content: -

Response:

- Status: 200 (OK)
- Content: json array of word strings, e.g. ['void', 'bike', ..., 'labor']

### Validate seed word

Validate a single word of keyvault entropy against the BIP39 word list.
Only the English dictionary is currently supported.

Request:

- Endpoint: POST `/bip39/validate-word`
- Parameters: -
- Headers: -
- Content: string containing a single seed word, i.e. a single item of the word array

Response:

- Status: 200
- Content: true/false (validity as bool)

### Validate seed phrase

Validate a whole seed phrase (i.e. BIP39 word list) as returned by [/bip39](#Generate-seed-phrase).

TODO Should we also return an error code or text describing the reason why validation failed?  

Request:

- Endpoint: POST `/bip39/validate-phrase`
- Parameters: -
- Headers: -
- Content: json array of word strings

Response:

- Status: 200
- Content: true/false (validity as bool)

## Vault initialization

Initialize a keyvault with a whole seed phrase (i.e. BIP39 word list) as returned by [/bip39](#Generate-seed-phrase).

Request:

- Endpoint: POST `/vault`
- Parameters: -
- Headers: -
- Content: json array of word strings

Response:

- Status: 201, 400 (wrong phrase)
- Content: -

## Profile management

### Restore all profiles

Restore all profiles into the initialized vault,
similarly to restoring a Bitcoin wallet from a seed phrase.
This mean regenerating profile keys/ids for your vault and checking for any sign of their usage.
This means trying to find public information or private backups of those profiles
on decentralized storage systems known by the server.

Request:

- Endpoint: POST `/vault/restore-dids`
- Parameters: -
- Headers: -
- Content: -

Response:

- Status: 201 or 409 (uninitialized vault)
- Content: object containing fields of resulted profile counts, e.g.

```json
{
    "try_count":23,    // Number of profiles checked for available information
    "restore_count":3, // Number of profiles successfully restored
},
```

### Get default profile

Query profile that is active by default and used by all profile-specific operations
unless an explicit profile id is given.

Request:

- Endpoint: GET `/vault/default-did`
- Parameters: -
- Headers: -
- Content: -

Response:

- Status: 200 or 409 (uninitialized vault)
- Content: profile id as a string or null if not set, e.g.

```json
"IezbeWGSY2dqcUBqT8K7R14xr"
```

### Set default profile

Set profile that is active by default.

Request:

- Endpoint: PUT `/vault/default-did`
- Parameters: -
- Headers: -
- Content: profile id as a string, e.g.

```json
"IezbeWGSY2dqcUBqT8K7R14xr"
```

Response:

- Status: 200 or 409 (uninitialized vault)
- Content: - 


### List all profiles

List all profiles that are already generated and present in the vault.

Request:

- Endpoint: GET `/vault/dids`
- Parameters: -
- Headers: -
- Content: -

Response:

- Status: 200 or 409 (uninitialized vault)
- Content: array of profile objects found, e.g.

```json
[
  {
    "id":"IezbeWGSY2dqcUBqT8K7R14xr",
    "label":"Mr Anderson",
    "avatar":"data:image/png;base64,iVBOR...",
    "state":"TODO",
  },
  {
    "id":"Iez25N5WZ1Q6TQpgpyYgiu9gTX",
    "label":"Neo",
    "avatar":"data:image/png;base64,iVBOR...",
    "state":"TODO",
  }
]
```

### Create new profile

Create an empty new profile by generating new private and public keys and a profile Id.

Request:

- Endpoint: POST `/vault/dids`
- Parameters: -
- Headers: -
- Content: -

Response:

- Status: 201 or 409 (uninitialized vault)
- Content: details of the newly created profile, e.g.

```json
{
  "id":"IezbeWGSY2dqcUBqT8K7R14xr",
  "label":"disco-deer",
  "avatar":"data:image/png;base64,iVBOR...",
  "state":"TODO",
}
```

### Load a single profile

Query details of a single profile that is already generated and present in the vault.
Note that using value `_` for the `did` path parameter in the query specifies
the profile that is active by default. 

Request:

- Endpoint: GET `/vault/dids/{did}`
- Parameters: -
- Headers: -
- Content: -

Response:

- Status: 200 or 409 (uninitialized vault)
- Content: the profile object found, e.g.

```json
{
  "id":"IezbeWGSY2dqcUBqT8K7R14xr",
  "label":"Mr Anderson",
  "avatar":"data:image/png;base64,iVBOR...",
  "state":"TODO",
}
```

### Rename profile

Specify a new label for an already existing profile.

Request:

- Endpoint: PUT `/vault/dids/{did}/label`
- Parameters: `did` is the identifier of an existing profile
- Headers: -
- Content: new label as string, e.g. `"Family"`

Response:

- Status: 200 or 409 (uninitialized vault)
- Content: -

### Change avatar picture

Update the avatar picture for an already existing profile.
**NOTE that currently only png images and base64 encoding is supported**.

Request:

- Endpoint: PUT `/vault/dids/{did}/avatar`
- Parameters: `did` is the identifier of an existing profile
- Headers: -
- Content: DataURI format of image as string, e.g. `"data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAIAAAACAAgMAAAC+UIlYAAAACVBMVEUJPcyM05NCbJvv7ERhAAAAcElEQVR4nO2UwQ2AMAwDw5AwJCxJjshSHmxgn2TaOvdEOc6qIldnc3eeTgQ/gQvnRl2E6VyEzgcPStj3CIOLwAMoWBrAMlEfYXARKBkSlbqTCBMXgY9gALuLMDgJDPTjAJ2WSIQ5XQQGPP5gFsFKeAHj8TBgukmquAAAAABJRU5ErkJggg=="`

Response:

- Status: 200, 409 (uninitialized vault) or 400 (wrong picture format) 
- Content: -

## Cryptography

### Sign claim

Sign a claim in the name of a specific profile. The request must have been created by the
`/vault/dids/{did}/claims/{claimid}/witness-request` endpoint,
but most likely in an different wallet and computer of another user.   

Request:

- Endpoint: POST `/vault/dids/{did}/sign-claim`
- Parameters: -
- Headers: -
- Content: opaque claim content string to be signed, e.g.
`uBjkmVk9cLFjWxFw1aPZuIGwSpsbvTC27f2wluRBKAioFufCHibJPkNTMfdGrKQ509tF5IfV3zSobfs3q4tbCCy6dTZJmjApsGKtP7HL9DheemW8vs2tDOsylXVLIRgNX46QP1fgRPpxGL0HZM7o5TojH44GqvKZqYh3Jy29CHiOnkl6xPfXht2EFhRLKuV4HsKEyEXKh2G5FHl3nq0bfv`

Response:

- Status: 200, 409 (uninitialized vault) or 400 (wrong claim format)
- Content: opaque claim witness signature string. Note that it has to internally contain the public key of the profile internally,
  validation would be impossible otherwise. E.g.
`u3OHYUwntzsAiG02THnSdsuRSDht2lA1TVATQPs8sYD9YA8nvVqSJNIB3je6NrlY5BnnZr3Kh954iAZ5nnZ4J2S7zCza10MScfZz2UAmorRe20ujZH82zFJ6CAdY8X8pOSxkWdoyaVLv4pJm0rPHheCngWY2mSqioz0GQGD85Hb6lfkyyl1gYytsIRswh594L91TyRwtYxxj7Ufg6pndlA2eEMD127whxVtG94YbrEMBCdhoiu0gDataTk52PHv1ycAb0QyTRsFU9ChPROXY8ZE4kujbR7VoMPCewAgwDcunGTWAeox1WBtiw4UHAH9mKhQYrvaUdNTDOFkMC0zqvh864TbkFjNxTbsEKPyAZOLHGh52pYuRl8Yn4yHVnuTy3k9IWTRFL3g8FB9AnczFVfgIc6HXU2Xpdb1AzT42MFmS2eCjiphKYQm3JtkrQwROK1aR9xdOywlAoBRqf5ZL21pC4W7TLZOx85K1sA1d8W0VKTvRDtxogGxeelQODtOObmIZqysKGfyznixtmenzfLZygllwuQvZ67HQcupxtDqZhZfQf1rBUzfm8CWsVXDWwRZDW0z0xCQDEwQDTqimLbXGZU3VFpaoSJJMyQVYuhlE7p0ZkJPMggNEHvcFvsO9KiShErgWjg0lcxhi7I6NsZRuXdRqGFLLc6P22WUJ0ToFmLbBIFufbD43XMt2iFzZ32iulsRwCj6mgbBUxG3R4oJgT0e9yq6jxsVWzFJYdTgnnEjgYoQu4Lqc8SjpQpr50c17EUBtb1TdeOgF9CvB7dzYOTTScqjLxlQKtEvlk5um6Qb98vWqMngQelyzsOjMgtDvOV1ARgR9qVWc2SHc6mFVC7AeV7emkMHtMiS0r9cM6x2UTGw2fqqierRcEBWTWdYKMRxLUyfcgFXV9yoq8QIrrzQ7eHo8djIodERM09Rtv1XUEalpvs7j1e4k6bC1vE2QOoZLL8fPm2LrOVeGqkz4q0jfWLFgPf1qf1JEkltsF2GcyyDUAdBRf4VSItTEj5ROGWtuMpMEaltGbyeMQUoKTOshPBdGt0Tf0HUll8ninRGqjKQQhGa2bHqr8WGmriUUSVVCFOZ1YYVWS2jodngoQzcar7TDk0aCbYHpXp6CQsmy0XMl4RzDB9wOa4mMue78xyBbsIuju87jqJyZr10XglKp0wlC9GxkLgBfJiGDksUbHeaY5vInss5kwVCJmpGetmrkXo1fQjecceuj9XAwmkAU3fVWS9MV0UEzIqqlfi07iKjpzs1j2teYdrrAPtE1BJbwgKGGB1DFDnWIteOflsmxQzU0jZsLFnetxtVob56UFTfXDeEGkvsiI8hykhaIKxqNCEIT8MbgUIvTKzQ5h7HeJlFywy3MYr1MQ7CSTKTXSogQc4vJq6WK7SddWs8hMR1ER7hpbIMtu0SaII8IwQ1tgOn0OahmvHGdoqxyXPhk8inJUu63xQ3cxuwZpcz8u6cVvd7LwzfquTaQ832wSsvrcFFttI6zWhEJq3YJHVimbs0ALhzgcl9cwx4G2THT0hZBWHJLhFyGuSevr25Ec7qqJnArF4WUifRZ0fDAMilL8XvgKP2HOkxY04Qtmcr3uuQMHwA7OxGYLeMxeiAfla0XT20MBsjb`

## Claims

### List all claims

List all claims that are present in the whole vault, regardless of their subject.

Request:

- Endpoint: GET `/vault/claims`
- Parameters: -
- Headers: -
- Content: -

Response:

- Status: 200 or 409 (uninitialized vault)
- Content: array of claim objects found in the vault, e.g.

```json
[{
  "id": "<claim_id>",
  "subject_id": "Iez24XMd3BfPn5LAJdGdvZp87n", // Morpheus DID
  "subject_label": "Neo",
  "schema_id": "<schema_id>",
  "schema_name": "age-over",
  "content": {
    // Completely schema-dependent structure here
  },
  "proof": [{
    "signer_id": "Iez24XMd3BfPn5LAJdGdvZp87n",
    "signed_message": {
      "public_key": "PezAgmjPHe5Qs4VakvXHGnd6NsYjaxt4suMUtf39TayrSfb",
      "message": "...",
      "signature": "...",
    }
  }],
}]
```

### List claims of a profile

List all claims that have a specified profile as their subject.

Request:

- Endpoint: GET `/vault/dids/{did}/claims`
- Parameters: -
- Headers: -
- Content: -

Response:

- Status: 200 or 409 (uninitialized vault)
- Content: array of claim objects found for the specified profile, e.g.

```json
[{
  "id": "<claim_id>",
  "subject_id": "Iez24XMd3BfPn5LAJdGdvZp87n", // Morpheus DID
  "subject_label": "Neo",
  "schema_id": "<schema_id>",
  "schema_name": "age-over",
  "content": {
    // Completely schema-dependent structure here
  },
  "proof": [],
}]
```

### Create a claim

Create a claim instance with given schema and contents for a specified profile as subject.

Request:

- Endpoint: POST `/vault/dids/{did}/claims`
- Parameters: -
- Headers: -
- Content: JSON object with details of the claim to be created, e.g.
```json
{
  "schema": "<content_hash_of_claim_schema>",
  "content": {
    // Completely schema-dependent structure here
  },
}
```

Response:

- Status: 201 or 409 (uninitialized vault)
- Content: id (content hash) string of the created claim  


### Request witness signature for a claim

Create a request message that contains a claim to be shared with a witness for signing. 

Request:

- Endpoint: GET `/vault/dids/{did}/claims/{claimid}/witness-request`
- Parameters: -
- Headers: -
- Content: -

Response:

- Status: 200 or 409 (uninitialized vault)
- Content: opaque claim string to be signed by a witness, e.g.
`uBjkmVk9cLFjWxFw1aPZuIGwSpsbvTC27f2wluRBKAioFufCHibJPkNTMfdGrKQ509tF5IfV3zSobfs3q4tbCCy6dTZJmjApsGKtP7HL9DheemW8vs2tDOsylXVLIRgNX46QP1fgRPpxGL0HZM7o5TojH44GqvKZqYh3Jy29CHiOnkl6xPfXht2EFhRLKuV4HsKEyEXKh2G5FHl3nq0bfv`

### Import witness signature for a claim

Validate witness signature for a claim and store it with the claim. 

Request:

- Endpoint: PUT `/vault/dids/{did}/claims/{claimid}/witness-signature`
- Parameters: -
- Headers: -
- Content: opaque signature string of a witness for the specified claim, e.g `u3OHYUwntzsAiG02THnSdsuRSDht2lA1TVATQPs8sYD9YA8nvVqSJNIB3je6NrlY5BnnZr3Kh954iAZ5nnZ4J2S7zCza10MScfZz2UAmorRe20ujZH82zFJ6CAdY8X8pOSxkWdoyaVLv4pJm0rPHheCngWY2mSqioz0GQGD85Hb6lfkyyl1gYytsIRswh594L91TyRwtYxxj7Ufg6pndlA2eEMD127whxVtG94YbrEMBCdhoiu0gDataTk52PHv1ycAb0QyTRsFU9ChPROXY8ZE4kujbR7VoMPCewAgwDcunGTWAeox1WBtiw4UHAH9mKhQYrvaUdNTDOFkMC0zqvh864TbkFjNxTbsEKPyAZOLHGh52pYuRl8Yn4yHVnuTy3k9IWTRFL3g8FB9AnczFVfgIc6HXU2Xpdb1AzT42MFmS2eCjiphKYQm3JtkrQwROK1aR9xdOywlAoBRqf5ZL21pC4W7TLZOx85K1sA1d8W0VKTvRDtxogGxeelQODtOObmIZqysKGfyznixtmenzfLZygllwuQvZ67HQcupxtDqZhZfQf1rBUzfm8CWsVXDWwRZDW0z0xCQDEwQDTqimLbXGZU3VFpaoSJJMyQVYuhlE7p0ZkJPMggNEHvcFvsO9KiShErgWjg0lcxhi7I6NsZRuXdRqGFLLc6P22WUJ0ToFmLbBIFufbD43XMt2iFzZ32iulsRwCj6mgbBUxG3R4oJgT0e9yq6jxsVWzFJYdTgnnEjgYoQu4Lqc8SjpQpr50c17EUBtb1TdeOgF9CvB7dzYOTTScqjLxlQKtEvlk5um6Qb98vWqMngQelyzsOjMgtDvOV1ARgR9qVWc2SHc6mFVC7AeV7emkMHtMiS0r9cM6x2UTGw2fqqierRcEBWTWdYKMRxLUyfcgFXV9yoq8QIrrzQ7eHo8djIodERM09Rtv1XUEalpvs7j1e4k6bC1vE2QOoZLL8fPm2LrOVeGqkz4q0jfWLFgPf1qf1JEkltsF2GcyyDUAdBRf4VSItTEj5ROGWtuMpMEaltGbyeMQUoKTOshPBdGt0Tf0HUll8ninRGqjKQQhGa2bHqr8WGmriUUSVVCFOZ1YYVWS2jodngoQzcar7TDk0aCbYHpXp6CQsmy0XMl4RzDB9wOa4mMue78xyBbsIuju87jqJyZr10XglKp0wlC9GxkLgBfJiGDksUbHeaY5vInss5kwVCJmpGetmrkXo1fQjecceuj9XAwmkAU3fVWS9MV0UEzIqqlfi07iKjpzs1j2teYdrrAPtE1BJbwgKGGB1DFDnWIteOflsmxQzU0jZsLFnetxtVob56UFTfXDeEGkvsiI8hykhaIKxqNCEIT8MbgUIvTKzQ5h7HeJlFywy3MYr1MQ7CSTKTXSogQc4vJq6WK7SddWs8hMR1ER7hpbIMtu0SaII8IwQ1tgOn0OahmvHGdoqxyXPhk8inJUu63xQ3cxuwZpcz8u6cVvd7LwzfquTaQ832wSsvrcFFttI6zWhEJq3YJHVimbs0ALhzgcl9cwx4G2THT0hZBWHJLhFyGuSevr25Ec7qqJnArF4WUifRZ0fDAMilL8XvgKP2HOkxY04Qtmcr3uuQMHwA7OxGYLeMxeiAfla0XT20MBsjb`

Response:

- Status: 201, 409 (uninitialized vault) or 400 (bad signature)
- Content: -  



### Create a presentation for a claim

TODO

### Load a single claim

TODO

### Update a claim

TODO what happens with related presentations?

### Delete a claim

Delete a claim instance with specified identifier.

Request:

- Endpoint: DELETE `/vault/dids/{did}/claims/{claim_id}`
- Parameters: -
- Headers: -
- Content: -

Response:

- Status: 200 or 409 (uninitialized vault)
- Content: -  


## Claim schemas

Each claim has to conform to a JSON schema that is identified by a content hash.

### List all claim schemas

This is a simplistic endpoint to retrieve all schemas with all metadat from the backend upon startup. We decided against a proper REST API with lazy retrieval of each schema by `id` after getting a list of metadata here.

Request:

- Endpoint: GET `/claim-schemas`
- Parameters: -
- Headers: -
- Content: -

Response:

- Status: 200
- Content: List of all schema with metadata, e.g.

```json

[{
    "id": "McL9746fWtE9EXV5",
    "label": "age-over",
    "content": {
        "type": "object",
        "properties": {
            "age": {
                "type": "number",
                "minimum": 0,
                "maximum": 255
            }
        }  
    },
    "ordering": ["age-over"]
}]
```

Id is a content hash of the whole schema excluding that single field. To avoid simple mistakes, the JSON document is normalized before calculating its hash.

The `ordering` top-level property is useful for the frontend to order editable fields on the user interface. Notice that JSON properties in an object have an undefined order. The array of property names needs to be treated quite liberally. There might be extra elements that are missing from the JSON Schema and there might be extra properties in the schema that have to be listed after all ordered properties. The only promises the backend make is that this `ordering` property will be an array of strings, but it might be potentially empty.
