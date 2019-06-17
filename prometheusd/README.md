# REST API for `prometheusd`

Daemon `prometheusd` accepts incoming REST client connections on `127.0.0.1:8080` by default. 

## Authentication and/or authorization

TODO

## Vault initialization

### <a name="/vault/generate_phrase"/> Generate seed phrase

Generate random entropy for a new keyvault, i.e. a new BIP39 "cold wallet" for the user.  

Request
* Endpoint: GET or POST `/vault/generate_phrase`
* Parameters: -
* Headers: -
* Content: - 

Response:
* Status: 200 (OK)
* Content: space-separated list of prase words in cleartext, e.g. `void bike outer blouse bronze thrive black sibling memory sudden maple tissue pink senior update tragic peace arm thrive boy virtual make mango labor`

### Validate seed word

Validate a single word of keyvault entropy against the BIP39 word list.
Only the English dictionary is currently supported.

Request
* Endpoint: GET or POST `/vault/validate_word/{word}`
* Parameters: URL path parameter `word` is a string containing a single seed word, i.e. a single item of the word list returned by the `generate_phrase` call 
* Headers: -
* Content: - 

Response:
* Status: 202 (Accepted) or 406 (Not Acceptable)
* Content: -


### Validate seed phrase

Validate a whole seed phrase (i.e. BIP39 word list) as returned by [/vault/generate_phrase](#/vault/generate_phrase).

Request
* Endpoint: GET or POST `/vault/validate_phrase/{phrase}`
* Parameters: URL path parameter `phrase` is a string containing a space-separated list of words as returned by the `generate_phrase` call
* Headers: -
* Content: -

Response:
* Status: 202 (Accepted) or 406 (Not Acceptable)
* Content: none if validation succeeded, error details in cleartext otherwise

## Profile management

TODO

## Claims

TODO