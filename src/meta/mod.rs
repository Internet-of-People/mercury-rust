use std::collections::HashMap;
use std::time::SystemTime;



pub trait HashSpaceLink
{
    fn hash(&self) -> &[u8];
    fn storage(&self) -> StorageId;
}

pub trait HashSpaceData
{
    fn hash(&self) -> &[u8]; // of blob data
    fn blob(&self) -> &[u8];
    fn attributes<'a>(&'a self) -> Box< Iterator< Item = &'a Box<Attribute + 'a> > + 'a >;
    // TODO add multicodec query here
    // fn format(&self) -> FormatId;
}

pub trait Attribute
{
    fn name(&self) -> &str;
    fn value(&self) -> &AttributeValue;
}


#[derive(Serialize, Deserialize)]
pub enum StorageId
{
    // TODO consider possible values
    InMemory,
    Postgres,
    LevelDb,
    Ipfs,
    StoreJ,
    Hydra,
}

#[derive(Serialize, Deserialize)]
pub struct GpsLocation
{
    latitude:   f64,
    longitude:  f64,
}

pub enum AttributeValue
{
    BOOLEAN(bool),
    INT(i64),
    FLOAT(f64),
    STRING(String),
    LOCATION(GpsLocation),
    TIMESTAMP(SystemTime),
    LINK( Box<HashSpaceLink> ),
    ARRAY( Vec< Box<AttributeValue> > ),
    OBJECT( HashMap< String, Box<AttributeValue> > ),
}



#[cfg(test)]
mod tests
{
    use super::*;
    //use common::imp::*;
    //use async::imp::*;


    // TODO
    //#[derive(Serialize, Deserialize)]
    struct MetaData
    {
        blob:       Vec<u8>,
        hash:       Vec<u8>,
        attributes: Vec< Box<Attribute> >,
    }

    impl HashSpaceData for MetaData
    {
        fn hash(&self) -> &[u8] { self.hash.as_ref() }
        fn blob(&self) -> &[u8] { self.blob.as_ref() }

        fn attributes<'a>(&'a self) -> Box< Iterator< Item = &'a Box<Attribute + 'a> > + 'a >
        {
            Box::new( self.attributes.iter() )
        }
    }


    #[test]
    fn test_metadata()
    {
        // TODO
    }
}