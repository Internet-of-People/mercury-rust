extern crate multihash;
extern crate serde;
extern crate serde_json;

use std::fmt;
use std::error::Error;

//use HashError;
//use Hasher;
//use Serializer;
//use MultiHasher;
//use SerdeJsonSerializer;
//use HashSpace as HashSpace;
//use HashSpaceError as HashSpaceError;
//use std::collections::HashMap;
//use serde::{Serialize, Deserialize};
//use std::marker::PhantomData;
//use std::ptr;
//use std::hash::Hash;
//use std::cmp::Eq;
//use StorageError as StorageError;
//use SerializerError;



#[derive(Debug)]
pub enum BlockChainError {
//    InvalidBlock,
//    BlockAlreadyThere,
//    ChangedChain,
    Other(Box<Error>),
}

impl fmt::Display for BlockChainError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str(Error::description(self))
    }
}

impl Error for BlockChainError {
    fn description(&self) -> &str {
        match *self {
//            BlockChainError::InvalidBlock  => "This block misses some parts, please fill before you try to push(Data, Hash)",
//            BlockChainError::BlockAlreadyThere  => "This block misses some parts, please fill before you try to push(Data, Hash)",
//            BlockChainError::ChangedChain   => "The chain has been modified somewhere",
            BlockChainError::Other(ref err)   => err.description(),
        }
    }
}


#[derive(Debug, Serialize, Deserialize)] //, Clone, PartialEq)]
pub struct Block<DataType, HashType>
{
    data:            DataType,
    prev_block_hash: HashType,
}



/*
enum StorageId
{
    TestInMemory,
    LevelDb,
    Ipfs,
    StoreJ,
    Hydra,
}

// TODO use this in a HashSpace implementation, probably CompositeHashSpace
struct MultiHash<ObjectType, StorageId, HashData>
{
    serializer: multicodec::Codec,
    hash_algo:  multihash::Hash,
    storage_id: StorageId,
    data:       HashData,
}
*/



//pub struct BlockChain<ObjectType, SerializedType, KeyType>
//{
//    serializer: Box< Serializer<ObjectType, SerializedType> >,
//    hasher:     Box< Hasher<SerializedType, KeyType> >,
//    pool: HashMap<KeyType, Block<ObjectType, KeyType>>,
//    last_key: KeyType,
//}
//
//impl<KeyType, SerializedType, ObjectType> BlockChain<ObjectType, SerializedType, KeyType>
//where KeyType: Eq + Hash + Clone,
//      ObjectType: Clone{
//    pub fn new( ser: Box< Serializer<ObjectType, SerializedType> >,
//            hash:     Box< Hasher<SerializedType, KeyType> >,
//            pool:       HashMap<KeyType, Block<ObjectType, KeyType>>,
//            last_key : KeyType
//            )-> Self{
//                BlockChain{
//                    serializer: ser,
//                    hasher: hash,
//                    pool: pool,
//                    last_key: last_key
//            }
//    }
//
//    fn store_block(&mut self, data: ObjectType, key : KeyType)->Result<bool, BlockChainError>{
//        let insertion = self.pool.insert(key.clone(), Block{data: data, prev_block_hash: self.last_key.clone()} );
//        if insertion.is_none(){
//            self.last_key=key;
//            Ok(true)
//        }
//        else{
//            Err(BlockChainError::BlockAlreadyThere)
//        }
//    }
//
//    pub fn serialize(&self, data : ObjectType) -> Result<SerializedType, SerializerError>{
//        self.serializer.serialize(&data)
//    }
//
//    pub fn deserialize(&self, data : SerializedType) -> Result<ObjectType, SerializerError>{
//        self.serializer.deserialize(&data)
//    }
//}
//
//impl<ObjectType, SerializedType, HashType> HashSpace<ObjectType, HashType>
//for BlockChain<ObjectType, SerializedType, HashType>
//where HashType: Eq + Hash + Clone,
//      ObjectType: Clone{
//    fn store(&mut self, object: ObjectType) -> Result<HashType, HashSpaceError>{
//        let ser = self.serializer.serialize(&object);
//        match ser{
//            Ok(serialized)=>{
//                let hash = self.hasher.hash(&serialized);
//                match hash {
//                    Ok(hash)=>{
//                        let success = self.store_block(object, hash.clone());
//                        match success {
//                            Ok(_)=>{
//                                Ok(hash)
//                            }
//                            Err(error)=>{
//                                Err(HashSpaceError::Other(Box::new(error)))
//                            }
//                        }
//                    },
//                    Err(error)=>{
//                        Err(HashSpaceError::HashError(error))
//                    }
//                }
//            },
//            Err(error)=>{
//                Err(HashSpaceError::SerializerError(error))
//            }
//        }
//    }
//
//    fn lookup(&self, hash: &HashType) -> Result<ObjectType, HashSpaceError>{
//        let stored = self.pool.get(&hash);
//        if stored.is_some(){
//            let stored_object = &stored.unwrap().data;
//            match self.serializer.serialize(&stored_object){
//                Ok(ser)=>{
//                    if self.hasher.validate(&ser, &hash).is_ok(){
//                        Ok(stored_object.clone())
//                    }else{
//                        Err(HashSpaceError::Other(Box::new(BlockChainError::ChangedChain)))
//                    }
//                },
//                Err(error)=>{
//                    Err(HashSpaceError::SerializerError(error))
//                }
//            }
//        }else{
//            Err(HashSpaceError::Other(Box::new(StorageError::InvalidKey)))
//        }
//    }
//}
