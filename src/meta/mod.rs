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
    Timestamp(SystemTime),
    Location(GpsLocation),
    String(&'a str),
    Link(&'a Link),
    Array(Box< 'a + Iterator< Item = AttributeValue<'a> > >),
    Object(Box< 'a + Iterator<Item = &'a Attribute> >),
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
        TIMESTAMP(SystemTime),
        LOCATION(GpsLocation),
        STRING(String),
        LINK(MetaLink),
        ARRAY( Vec<MetaAttrVal> ),
        OBJECT( Vec<MetaAttr> ),
    }

    impl MetaAttrVal
    {
        fn to_attr_val<'a>(&'a self) -> AttributeValue<'a>
        {
            match *self {
                MetaAttrVal::BOOL(v)            => AttributeValue::Boolean(v),
                MetaAttrVal::INT(v)             => AttributeValue::Integer(v),
                MetaAttrVal::FLOAT(v)           => AttributeValue::Float(v),
                MetaAttrVal::TIMESTAMP(v)       => AttributeValue::Timestamp(v),
                MetaAttrVal::LOCATION(v)        => AttributeValue::Location(v),
                MetaAttrVal::STRING(ref v)      => AttributeValue::String(&v),
                MetaAttrVal::LINK(ref v)        => AttributeValue::Link(v),
                MetaAttrVal::ARRAY(ref v)       => AttributeValue::Array(
                    Box::new( v.iter().map( |m| m.to_attr_val() ) ) ),
                MetaAttrVal::OBJECT(ref v)      => AttributeValue::Object(
                    Box::new( v.iter().map( |m| m as &Attribute) ) ),
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
        let spoon = "There is no Rust";
        let answer = 42;
        let pi = 3.14159265358979;

        let linkhash = b"Far far away in another storage network".to_vec();
        let famous = vec!(
            MetaAttrVal::STRING( spoon.to_owned() ),
            MetaAttrVal::INT(answer),
            MetaAttrVal::FLOAT(pi),
        );
        let color = vec!(
            MetaAttr::new( "red", MetaAttrVal::INT(90) ),
            MetaAttr::new( "green", MetaAttrVal::INT(60) ),
            MetaAttr::new( "blue", MetaAttrVal::INT(90) ),
        );
        let attrs = vec!(
            MetaAttr::new( "works", MetaAttrVal::BOOL(true) ),
            MetaAttr::new( "timestamp", MetaAttrVal::TIMESTAMP( SystemTime::now() ) ),
            MetaAttr::new( "link", MetaAttrVal::LINK( MetaLink::new(linkhash, StorageId::InMemory) ) ),
            MetaAttr::new( "famous", MetaAttrVal::ARRAY(famous) ),
            MetaAttr::new( "color", MetaAttrVal::OBJECT(color) ),
        );
        let blob = b"1234567890abcdef".to_vec();
        let hash = b"qwerty".to_vec();
        let metadata = MetaData::new(blob, hash, attrs);

        {
            // Test works bool attribute
            let works_attrs: Vec<&Attribute> = metadata.attributes()
                .filter(|attr| attr.name() == "works")
                .collect();
            assert_eq!(works_attrs.len(), 1);
            let works_attr = works_attrs.get(0).unwrap();

            assert_eq!(works_attr.name(), "works");
            let works_val = match works_attr.value() {
                AttributeValue::Boolean(v) => v,
                _ => panic!("Unexpected attribute type"),
            };
            assert!(works_val);
        }

        {
            // Test color object attribute
            let fame_attrs: Vec<&Attribute> = metadata.attributes()
                .filter(|attr| attr.name() == "famous")
                .collect();
            assert_eq!(fame_attrs.len(), 1);
            let fame_attr = fame_attrs.get(0).unwrap();

            assert_eq!(fame_attr.name(), "famous");
            let fame_value: Vec<AttributeValue> = match fame_attr.value() {
                AttributeValue::Array(v) => v.collect(),
                _ => panic!("Unexpected attribute type"),
            };
            assert_eq!( fame_value.len(), 3 );
// TODO implement asserts for array element values
//            assert_eq!( fame_value[0], AttributeValue::String(spoon) );
//            assert_eq!( fame_value[1], AttributeValue::Integer(answer) );
//            assert_eq!( fame_value[2], AttributeValue::Float(pi) );
        }

        {
            // Test color object attribute
            let color_attrs: Vec<&Attribute> = metadata.attributes()
                .filter(|attr| attr.name() == "color")
                .collect();
            assert_eq!(color_attrs.len(), 1);
            let color_attr = color_attrs.get(0).unwrap();

            assert_eq!(color_attr.name(), "color");
            let color_value = match color_attr.value() {
                AttributeValue::Object(v) => v,
                _ => panic!("Unexpected attribute type"),
            };

// TODO write asserts for color fields
        }
    }
}
