use error::*;
use meta;
use meta::{Attribute, AttributeValue};

pub mod imp;



pub type StorageId = String;

//#[derive(Debug, Copy, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
//pub enum StorageId
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


pub trait Link
{
    fn storage(&self) -> &StorageId;
    fn hash(&self)    -> &[u8];         // of linked data under specified storage
    fn sublink(&self) -> Option<&Link>; // relative path, analogue to URL resource
}



pub trait Data
{
    fn blob(&self) -> &[u8];
    fn attributes<'a>(&'a self) -> Box< 'a + Iterator<Item = &'a Attribute> >;

    // Convenience function to access attributes by name/path
    fn first_attrval_by_name<'a>(&'a self, name: &str)
            -> Option< AttributeValue<'a> >
        { meta::iter_first_attrval_by_name( self.attributes(), name ) }

    fn first_attrval_by_path<'a>(&'a self, path: &[&str])
            -> Option< AttributeValue<'a> >
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

