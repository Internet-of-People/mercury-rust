# REST API for `prometheusd`

Daemon `prometheusd` accepts incoming REST client connections on `127.0.0.1:8080` by default.
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
  - [Create new schema (not implemented)](#Create-new-schema-not-implemented)
  - [Create new version of a given schema (not implemented)](#Create-new-version-of-a-given-schema-not-implemented)
  - [Get latest version of a given schema](#Get-latest-version-of-a-given-schema)
  - [Get given version of a given schema](#Get-given-version-of-a-given-schema)

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
    "alias":"Mr Anderson",
    "avatar":"data:image/png;base64,iVBOR...",
    "state":"TODO",
  },
  {
    "id":"Iez25N5WZ1Q6TQpgpyYgiu9gTX",
    "alias":"Neo",
    "avatar":"data:image/png;base64,iVBOR",
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
  "alias":"Mr Anderson",
  "avatar":"data:image/png;base64,iVBOR...",
  "state":"TODO",
}
```

### Create new profile

List all profiles that are already generated and present in the vault.

TODO Consolidate with `POST /claim-schemas` so either this gives 303 to the created object or that
also includes the details of the created object.

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
  "alias":"disco-deer",
  "avatar":"data:image/png;base64,iVBOR...",
  "state":"TODO",
}
```

### Rename profile

Specify a new alias for an already existing profile.

TODO should we join name and avatar updates into a single update operation as standard REST usually does?

Request:

- Endpoint: PUT `/vault/dids/{did}/alias`
- Parameters: `did` is the identifier of an existing profile
- Headers: -
- Content: new alias as string, e.g. `"Family"`

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

TODO

- Endpoint: GET `/vault/claims/...`

## Claim schemas

### List all claim schemas

TODO Consolidate with `GET /vault/dids` so either both expand objects, or both return just links to objects as REST defines.

Request:

- Endpoint: GET `/claim-schemas`
- Parameters: -
- Headers: -
- Content: -

Response:

- Status: 200
- Content: List of schema identifiers, e.g.

```json
["/claim-schemas/McL9746fWtE9EXV5", "/claim-schemas/sU7TNXQjhcUWLw3c", "/claim-schemas/cagtn4rDCHUyqigF"]
```

### Create new schema (not implemented)

TODO Provide authentication to prove rights to author the schema.

Request:

- Endpoint: POST `/claim-schemas`
- Parameters: -
- Headers: -
- Content: Content and metadata of the schema, e.g.

```json
{
  name: "age-over",
  author: "iop",
  contents: {
      "type": "object",
      "properties": {
          "age": {
              "type": "number",
              "minimum": 0,
              "maximum": 255
          }
      }
  }
}
```

Response:

- Status: 303 or 403 (unauthorized)
- Headers
  - Location: Link to the created object, e.g. `Location: /claim-schemas/McL9746fWtE9EXV5/0`
- Content: Empty

### Create new version of a given schema (not implemented)

TODO Provide authentication to prove rights to author the schema.

Request:

- Endpoint: POST `/claim-schemas/{id}`
- Parameters: `id` is the artificial identifier of the schema
- Headers: -
- Content: New content of the schema, e.g.

```json
{
    "type": "object",
    "properties": {
        "age": {
            "type": "number",
            "minimum": 0,
            "maximum": 255
        }
    }
}
```

Response:

- Status: 303 or 403 (unauthorized)
- Headers
  - Location: Link to the created object, e.g. `Location: /claim-schemas/McL9746fWtE9EXV5/1`
- Content: Empty

### Get latest version of a given schema

Request:

- Endpoint: GET `/claim-schemas/{id}/latest`
- Parameters: `id` is the artificial identifier of the schema
- Headers: -
- Content: -

Response:

- Status: 302 (temporary redirect) or 404 (id not found)
- Headers:
  - Location: Link to the latest version, e.g. `Location: /claim-schemas/McL9746fWtE9EXV5/1`
- Content: Empty

### Get given version of a given schema

Request:

- Endpoint: GET `/claim-schemas/{id}/{version}`
- Parameters: `id` is the artificial identifier of the schema
- Headers: -
- Content: -

Response:

- Status: 200 or 404 (id is not found or invalid version)
- Headers:
  - `Link: </claim/schemas/{id}/{version}>; rel="latest-version"`
- Content: Contents and metadata of the given version of the given schema, e.g.

```json
{
  name: "age-over",
  author: "iop",
  version: 0,
  contents: {
      "$id": "/claim-schemas/McL9746fWtE9EXV5/0",
      "type": "object",
      "properties": {
          "age": {
              "type": "number"
          }
      }
  }
}
```
