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

- Status: 201 or 400 (wrong phrase)
- Content: -

## Profile management

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

### Load a single profile

Query details of a single profile that is already generated and present in the vault.

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

### Create new profile

List all profiles that are already generated and present in the vault.

Request:

- Endpoint: POST `/vault/dids`
- Parameters: -
- Headers: -
- Content: -

Response:

- Status: 201 or 409 (uninitialized vault)
- Content: details of the newly created DID object, e.g.

```json
{
  "id":"IezbeWGSY2dqcUBqT8K7R14xr",
  "label":"disco-deer",
  "avatar":"data:image/png;base64,iVBOR...",
  "state":"TODO",
}
```

### Rename profile

Specify a new label for an already existing profile.

TODO should we join name and avatar updates into a single update operation as standard REST usually does?

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

- Status: 200 or 409 (uninitialized vault)
- Content: -

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
  "proof": [ "TODO" ],
  "presentation": [ "TODO" ]
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
  "proof": [ "TODO" ],
  "presentation": [ "TODO" ]
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

TODO

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
