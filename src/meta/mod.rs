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
    fn attributes<'a>(&'a self) -> Box< Iterator<Item = &'a Attribute> + 'a >;
    // TODO add multicodec query here
    // fn format(&self) -> FormatId;
}

pub trait Attribute
{
    fn name(&self)  -> &str;
    fn value<'a>(&'a self) -> AttributeValue<'a>;
}


pub enum AttributeValue<'a>
{
    Boolean(bool),
    Integer(i64),
    Float(f64),
    Location(GpsLocation),
    String(&'a str),
    Timestamp(&'a SystemTime),
    Link(&'a Link),
    Array(&'a Iterator< Item = AttributeValue<'a> >),
    Object(&'a Iterator<Item = &'a Attribute>),
}


#[derive(Debug, Copy, Clone, Serialize, Deserialize, PartialEq)]
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

#[derive(Debug, Copy, Clone, Serialize, Deserialize, PartialEq)]
pub struct GpsLocation
{
    latitude:   f64,
    longitude:  f64,
}




#[cfg(test)]
mod tests
{
    use super::*;
    //use common::imp::*;
    //use async::imp::*;


    #[derive(Debug, PartialEq, Serialize, Deserialize)]
    struct MetaLink
    {
        hash:    Vec<u8>,
        storage: StorageId,
    }

    impl MetaLink
    {
        fn new(hash: Vec<u8>, storage: StorageId) -> Self
            { Self{hash: hash, storage: storage} }
    }

    impl Link for MetaLink
    {
        fn hash(&self)    -> &[u8]      { self.hash.as_ref() }
        fn storage(&self) -> StorageId  { self.storage }
    }


    #[derive(Debug, Serialize, Deserialize, PartialEq)]
    enum MetaAttrVal
    {
        BOOL(bool),
        INT(i64),
        FLOAT(f64),
        LOCATION(GpsLocation),
        STRING(String),
        TIMESTAMP(SystemTime),
        LINK(MetaLink),
//        ARRAY( Vec<MetaAttrVal> ),
//        OBJECT( Vec<MetaAttr> ),
    }

    impl MetaAttrVal
    {
        fn to_attr_val<'a>(&'a self) -> AttributeValue<'a>
        {
            match *self {
                MetaAttrVal::BOOL(v)            => AttributeValue::Boolean(v),
                MetaAttrVal::INT(v)             => AttributeValue::Integer(v),
                MetaAttrVal::FLOAT(v)           => AttributeValue::Float(v),
                MetaAttrVal::LOCATION(v)        => AttributeValue::Location(v),
                MetaAttrVal::STRING(ref v)      => AttributeValue::String(&v),
                MetaAttrVal::TIMESTAMP(ref v)   => AttributeValue::Timestamp(&v),
                MetaAttrVal::LINK(ref v)        => AttributeValue::Link(v),
//                MetaAttrVal::ARRAY(ref v)       => AttributeValue::Array( &v.iter().map( |m| m.to_attr_val() ) ),
//                MetaAttrVal::OBJECT(ref v)      => AttributeValue::Object( &v.iter().map( |m| m as &Attribute) ),
            }
        }
    }



    #[derive(Debug, Serialize, Deserialize, PartialEq)]
    struct MetaAttr
    {
        name:   String,
        value:  MetaAttrVal,
    }

    impl MetaAttr
    {
        pub fn new(name: &str, value: MetaAttrVal) -> Self
            { Self{ name: name.to_owned(), value: value } }
    }

    impl Attribute for MetaAttr
    {
        fn name(&self) -> &str { &self.name }

        fn value<'a>(&'a self) -> AttributeValue<'a>
            { self.value.to_attr_val() }
    }



    #[derive(Debug, Serialize, Deserialize)]
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

        fn attributes<'a>(&'a self) -> Box< Iterator<Item = &'a Attribute> + 'a >
        {
            let result = self.attrs.iter().map( |meta| meta as &Attribute );
            Box::new(result)
        }
    }



    #[test]
    fn test_metadata()
    {
        let linkhash = b"Far far away in another storage network".to_vec();
        let attrs = vec!(
            MetaAttr::new( "works", MetaAttrVal::BOOL(true) ),
            MetaAttr::new( "timestamp", MetaAttrVal::TIMESTAMP( SystemTime::now() ) ),
            MetaAttr::new( "link", MetaAttrVal::LINK( MetaLink::new(linkhash, StorageId::InMemory) ) ),
        );
        let blob = b"1234567890abcdef".to_vec();
        let hash = b"qwerty".to_vec();
        let metadata = MetaData::new(blob, hash, attrs);

        let test_attr : Vec<&Attribute> = metadata.attributes()
            .filter( |attr| attr.name() == "works" )
            .collect();
        assert_eq!( test_attr.len(), 1 );
        assert_eq!( test_attr.get(0).unwrap().name(), "works" );

        match test_attr.get(0).unwrap().value() {
            AttributeValue::Boolean(v) => assert!(v),
            _ => assert!(false),
        }
    }
}