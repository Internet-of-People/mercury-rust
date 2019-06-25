# REST API for `prometheusd`

Daemon `prometheusd` accepts incoming REST client connections on `127.0.0.1:8080` by default.
This can be overridden using configuration option `--listen IP:PORT`.



## Authentication and/or authorization

TODO



## BIP39 seed phrases


### <a name="/bip39/new"/> Generate seed phrase

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

Validate a whole seed phrase (i.e. BIP39 word list) as returned by [/bip39/new](#/bip39/new).

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

TODO



## Profile management

### List all profiles

List all profiles that are already generated and present in the vault.

Request
* Endpoint: GET `/vault/dids`
* Parameters: - 
* Headers: -
* Content: -

Response:
* Status: 200
* Content: array of profile objects found, e.g. `["IezbeWGSY2dqcUBqT8K7R14xr","Iez25N5WZ1Q6TQpgpyYgiu9gTX"]`

TODO content should be something like
```json
[
  {
	"id": "multihash string",
	"alias": "string",
	"avatar": "TODO",
	"status": "TODO",
  },
  { ... }
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
* Status: 200
* Content: the newly created DID multihash, e.g. `"Iez25N5WZ1Q6TQpgpyYgiu9gTX"`


## Claims

TODO