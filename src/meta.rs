use std::collections::HashMap;
use std::time::SystemTime;



pub enum StorageId
{
    // TODO consider possible values
    InMemoryStorage,
    PostgresStorage,
    IpfsStorage,
    StoreJStorage,
}

pub enum FormatId
{
    TODO, // TODO
}

pub trait HashSpaceLink
{
    fn hash(&self) -> &[u8];
    fn storage(&self) -> StorageId;
}

pub struct GpsLocation
{
    latitude:   f64,
    longitude:  f64,
}

pub enum Attribute
{
    BOOLEAN(bool),
    INT(i64),
    FLOAT(f64),
    STRING(String),
    LOCATION(GpsLocation),
    TIMESTAMP(SystemTime),
    ARRAY( Vec< Box<Attribute> > ),
    OBJECT( HashMap< String, Box<Attribute> > ),
    LINK( Box<HashSpaceLink> ),
}


pub trait HashSpaceData
{
    fn hash(&self) -> &[u8];
    // fn size(&self) -> u64;
    fn data(&self) -> &[u8];
    fn attributes(&self) -> HashMap<String, Attribute>;
}
