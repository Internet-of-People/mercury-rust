use error::*;

pub mod imp;



pub trait Serializer<ObjectType, SerializedType>
{
    // TODO error handling: these two operations could return different error types
    //      (SerErr/DeserErr), consider if that might be clearer
    fn serialize(&self, object: &ObjectType) -> Result<SerializedType, SerializerError>;
    fn deserialize(&self, serialized_object: &SerializedType) -> Result<ObjectType, SerializerError>;
}

pub trait Hasher<ObjectType, HashType>
{
    // TODO should (maybe in a different trait?) differentiate between
    //      calculated binary hash and its multibase string representation
    fn get_hash(&self, object: &ObjectType) -> Result<HashType, HashError>;
    fn validate(&self, object: &ObjectType, hash: &HashType) -> Result<bool, HashError>;
}

pub trait StringCoder<HashType>
{
    fn encode(&self, hash_bytes: &HashType) -> Result<String, StringCoderError>;
    fn decode(&self, hash_str: &str) -> Result<HashType, StringCoderError>;
}

