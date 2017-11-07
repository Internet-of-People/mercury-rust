use std::collections::HashMap;
use std::time::SystemTime;



pub trait Link
{
    fn hash(&self) -> &[u8];
    fn storage(&self) -> StorageId;
}

pub trait Data
{
    fn hash(&self) -> &[u8]; // of blob data
    fn blob(&self) -> &[u8];
    fn attributes<'a>(&'a self) -> Box< Iterator< Item = &'a (Attribute + 'a) > + 'a >;
    // TODO add multicodec query here
    // fn format(&self) -> FormatId;
}

pub trait Attribute
{
    fn name(&self)  -> &str;
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

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct GpsLocation
{
    latitude:   f64,
    longitude:  f64,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub enum AttributeValue
{
    BOOLEAN(bool),
    INT(i64),
    FLOAT(f64),
    STRING(String),
    LOCATION(GpsLocation),
    TIMESTAMP(SystemTime),
//    LINK( Box<Link> ),
//    ARRAY( Vec< Box<AttributeValue> > ),
//    OBJECT( HashMap< String, Box<AttributeValue> > ),
}



#[cfg(test)]
mod tests
{
    use super::*;
    //use common::imp::*;
    //use async::imp::*;



    #[derive(Debug, Serialize, Deserialize)]
    struct MetaAttr
    {
        name:   String,
        value:  AttributeValue,
    }

    impl MetaAttr
    {
        pub fn new(name: &str, value: AttributeValue) -> Self
            { Self{ name: name.to_owned(), value: value } }
    }

    impl Attribute for MetaAttr
    {
        fn name(&self)  -> &str             { &self.name }
        fn value(&self) -> &AttributeValue  { &self.value }
    }



    //#[derive(Debug, Serialize, Deserialize)]
    struct MetaData
    {
        blob:   Vec<u8>,
        hash:   Vec<u8>,
        attrs:  Vec<MetaAttr>,
    }

    impl MetaData
    {
        pub fn new(blob: Vec<u8>, hash: Vec<u8>,
                   attrs: Vec<MetaAttr>) -> Self
            { Self{ blob: blob, hash: hash, attrs: attrs } }
    }

    impl Data for MetaData
    {
        fn hash(&self) -> &[u8] { self.hash.as_ref() }
        fn blob(&self) -> &[u8] { self.blob.as_ref() }

        fn attributes<'a>(&'a self) -> Box< Iterator< Item = &'a (Attribute + 'a) > + 'a >
        {
            let result = self.attrs.iter().map( |meta| meta as &Attribute );
            Box::new(result)

        }
    }



    #[test]
    fn test_metadata()
    {
        let attrs = vec!(
            MetaAttr::new( "test", AttributeValue::BOOLEAN(true) ),
            MetaAttr::new( "timestamp", AttributeValue::TIMESTAMP( SystemTime::now() ) ),
        );
        let blob = b"1234567890abcdef".to_vec();
        let hash = b"qwerty".to_vec();
        let metadata = MetaData::new(blob, hash, attrs);

        let test_attr : Vec<&Attribute> = metadata.attributes()
            .filter( |attr| attr.name() == "test" )
            .collect();
        assert_eq!( test_attr.len(), 1 );
        assert_eq!( test_attr.get(0).unwrap().name(), "test" );
        assert_eq!( *test_attr.get(0).unwrap().value(), AttributeValue::BOOLEAN(true) );
    }
}