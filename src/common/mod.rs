use error::*;
use meta;
use meta::{Attribute, AttributeValue};

pub mod imp;



pub type HashSpaceId = String;

//#[derive(Debug, Copy, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
//pub enum HashSpaceId
//{
//    // TODO consider possible values
//    Relative,
//    Postgres,
//    LevelDb,
//    Magnet,
//    Torrent,
//    Ipfs,
//    StoreJ,
//    Git,
//    Hydra,
//}


//pub trait HashLink
//{
//    fn hashspace(&self) -> &HashSpaceId;
//    fn hash(&self)      -> &str;          // of linked data under specified hashspace
//}



#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct HashWebLink
{
    hashspace:  HashSpaceId,
    hash:       String,
}

impl HashWebLink
{
    // TODO solve using &str instead of &String
    pub fn new(hashspace: &HashSpaceId, hash: &str) -> Self
        { Self{ hashspace: hashspace.to_owned(), hash: hash.to_owned() } }

    pub fn hashspace(&self) -> &HashSpaceId { &self.hashspace }
    pub fn hash(&self)      -> &str         {  self.hash.as_ref() }

    pub fn parse(address_str: &str)
        -> Result<HashWebLink, HashSpaceError>
    {
        // Ignore starting slash
        let address = if address_str.starts_with('/') { &address_str[1..] } else { address_str };

        // Split hashspaceId and hash parts
        let slash_pos = address.find('/')
            .ok_or( HashSpaceError::LinkFormatError( address_str.to_owned() ) )?; //.unwrap_or( address.len() );
        let (hashspace_id, slashed_hash) = address.split_at(slash_pos);
        let hash = &slashed_hash[1..]; // Ignore starting slash

        // Perform link resolution
        let hashlink = HashWebLink::new( &hashspace_id.to_string(), hash );
        Ok(hashlink)
    }
}



pub trait Data
{
    fn blob(&self) -> &[u8];
    fn attributes<'a>(&'a self) -> Box< 'a + Iterator<Item = &Attribute> >;

    // Convenience function to access attributes by name/path
    fn first_attrval_by_name(&self, name: &str)
            -> Option<AttributeValue>
        { meta::iter_first_attrval_by_name( self.attributes(), name ) }

    fn first_attrval_by_path(&self, path: &[&str])
            -> Option<AttributeValue>
        { meta::iter_first_attrval_by_path( self.attributes(), path ) }
}



// De/Serialize in-memory data from/to a memory-independent storable
// (binary, e.g. bson or json-utf8) representation
pub trait Serializer<ObjectType, SerializedType>
{
    // TODO error handling: these two operations could return different error types
    //      (SerErr/DeserErr), consider if that might be clearer
    fn serialize(&self, object: ObjectType) -> Result<SerializedType, SerializerError>;
    fn deserialize(&self, serialized_object: SerializedType) -> Result<ObjectType, SerializerError>;
}

// Provide (binary, e.g. SHA2) hash for (binary) data and validate hash against data
pub trait Hasher<ObjectType, HashType>
{
    fn get_hash(&self, object: &ObjectType) -> Result<HashType, HashError>;
    fn validate(&self, object: &ObjectType, hash: &HashType) -> Result<bool, HashError>;
}

// Provide human-readable (e.g. Base64) representation of (binary) hashes
pub trait HashCoder<BinaryHashType, ReadableHashType>
{
    fn encode(&self, hash_bytes: &BinaryHashType)
        -> Result<ReadableHashType, StringCoderError>;
    fn decode(&self, hash_str: &ReadableHashType)
        -> Result<BinaryHashType, StringCoderError>;
}

