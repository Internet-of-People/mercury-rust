# REST API for `prometheusd`

Daemon `prometheusd` accepts incoming REST client connections on `127.0.0.1:8080` by default.
This can be overridden using configuration option `--listen IP:PORT`.



## Authentication and/or authorization

TODO



## BIP39 seed phrases


### <a name="/bip39"/> Generate seed phrase

Generate random entropy for a new keyvault, i.e. a new BIP39 "cold wallet" for the user.  

Request
* Endpoint: POST `/bip39`
* Parameters: -
* Headers: -
* Content: - 

Response:
* Status: 200 (OK)
* Content: json array of word strings, e.g. ['void', 'bike', ..., 'labor']


### Validate seed word

Validate a single word of keyvault entropy against the BIP39 word list.
Only the English dictionary is currently supported.

Request
* Endpoint: POST `/bip39/validate_word`
* Parameters: - 
* Headers: -
* Content: string containing a single seed word, i.e. a single item of the word array

Response:
* Status: 200
* Content: true/false (validity as bool)


### Validate seed phrase

Validate a whole seed phrase (i.e. BIP39 word list) as returned by [/bip39](#/bip39).

TODO Should we also return an error code or text describing the reason why validation failed?  

Request
* Endpoint: POST `/bip39/validate_phrase`
* Parameters: -
* Headers: -
* Content: json array of word strings

Response:
* Status: 200
* Content: true/false (validity as bool)



## Vault initialization

Initialize a keyvault with a whole seed phrase (i.e. BIP39 word list) as returned by [/bip39](#/bip39).

Request
* Endpoint: POST `/vault`
* Parameters: -
* Headers: -
* Content: json array of word strings

Response:
* Status: 201 or 400 (wrong phrase)
* Content: -



## Profile management

### List all profiles

List all profiles that are already generated and present in the vault.

Request
* Endpoint: GET `/vault/dids`
* Parameters: - 
* Headers: -
* Content: -

Response:
* Status: 200 or 409 (uninitialized vault)
* Content: array of profile objects found, e.g.
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


### Create new profile

List all profiles that are already generated and present in the vault.

Request
* Endpoint: POST `/vault/dids`
* Parameters: - 
* Headers: -
* Content: -

Response:
* Status: 201 or 409 (uninitialized vault)
* Content: details of the newly created DID object, e.g.
```json
{
  "id":"IezbeWGSY2dqcUBqT8K7R14xr",
  "alias":"disco-deer",
  "avatar":"data:image/png;base64,iVBOR",
  "state":"TODO",
}
```

### Rename profile

Specify a new alias for an already existing profile.

TODO: should this use PUT instead to be more REST conformant? 

Request
* Endpoint: PUT `/vault/dids/{did}/alias`
* Parameters: `did` is the identifier of an existing profile
* Headers: -
* Content: new alias as string, e.g. "Family"

Response:
* Status: 200 or 409 (uninitialized vault)
* Content: -



## Claims

TODO