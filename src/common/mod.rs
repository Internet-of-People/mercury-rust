use error::*;

pub mod imp;



pub trait Link
{
    fn hash(&self) -> &[u8]; // of linked data
    fn storage(&self) -> StorageId;
    fn format(&self) -> FormatId;
}

pub trait Blob
{
    fn blob(&self) -> &[u8];
}



#[derive(Debug, Copy, Clone, Serialize, Deserialize, PartialEq)]
pub enum StorageId
{
    // TODO consider possible values
    InMemory,
    Postgres,
    LevelDb,
    Torrent,
    Ipfs,
    StoreJ,
    Hydra,
}


#[derive(Debug, Copy, Clone, Serialize, Deserialize, PartialEq)]
pub enum FormatId
{
    Torrent,
    Ipfs,
    Git,
    StoreJ,
    // TODO what others? ...
}



pub trait Serializer<ObjectType, SerializedType>
{
    // TODO error handling: these two operations could return different error types
    //      (SerErr/DeserErr), consider if that might be clearer
    fn serialize(&self, object: ObjectType) -> Result<SerializedType, SerializerError>;
    fn deserialize(&self, serialized_object: SerializedType) -> Result<ObjectType, SerializerError>;
}

pub trait Hasher<ObjectType, HashType>
{
    fn get_hash(&self, object: &ObjectType) -> Result<HashType, HashError>;
    fn validate(&self, object: &ObjectType, hash: &HashType) -> Result<bool, HashError>;
}

pub trait StringCoder<HashType>
{
    fn encode(&self, hash_bytes: &HashType) -> Result<String, StringCoderError>;
    fn decode(&self, hash_str: &str) -> Result<HashType, StringCoderError>;
}

