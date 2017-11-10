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
    fn attributes<'a>(&'a self) -> Box< 'a + Iterator<Item = &'a Attribute> >;
    // TODO add multicodec query here
    // fn format(&self) -> FormatId;

    // TODO implement these convenience functions outside this trait
    fn first_attrval_by_name<'a>(&'a self, name: &str)
            -> Option< AttributeValue<'a> >
        { self.iter_first_attrval_by_name( self.attributes(), name ) }

    fn first_attrval_by_path<'a>(&'a self, path: &[&str])
            -> Option< AttributeValue<'a> >
        { self.iter_first_attrval_by_path( self.attributes(), path ) }

    fn iter_first_attrval_by_name<'a>(&'a self, iter: Box< 'a + Iterator<Item = &'a Attribute> >, name: &str)
        -> Option< AttributeValue<'a> >
    {
        iter.filter( |attr| attr.name() == name )
            .nth(0)
            .map( |attr| attr.value() )
    }

    fn iter_first_attrval_by_path<'a>(&'a self, iter: Box< 'a + Iterator<Item = &'a Attribute> >, path: &[&str])
        -> Option< AttributeValue<'a> >
    {
        if path.len() == 0
            { return None; }

        let first_attrval = self.iter_first_attrval_by_name( iter, path[0] );
        if path.len() == 1
            { return first_attrval; }

        if let None = first_attrval
            { return None; }
        match first_attrval.unwrap() {
            AttributeValue::Object(attrs) => self.iter_first_attrval_by_path( attrs, &path[1..] ),
            _ => None,
        }
    }
}

pub trait Attribute
{
    fn name(&self)  -> &str;
    fn value<'a>(&'a self) -> AttributeValue<'a>;
}


// TODO this currently cannot easily be made PartialEq because of refs/boxes.
//      Now we need match to compare/extract values, it could be more convenient.
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
            // Test simple bool attribute
            let works_attrval = metadata.first_attrval_by_name("works");
            match works_attrval.unwrap() {
                AttributeValue::Boolean(v) => assert!(v),
                _ => panic!("Unexpected attribute type"),
            };
        }

        {
            // Test array attribute
            let fame_attrval = metadata.first_attrval_by_name("famous");
            match fame_attrval.unwrap() {
                AttributeValue::Array(v) => {
                    let arr: Vec<AttributeValue> = v.collect();
                    assert_eq!( arr.len(), 3 );
                    match arr[0] {
                        AttributeValue::String(val) => assert_eq!(val, spoon),
                        _ => panic!("Unexpected attribute type"),
                    };
                    match arr[1] {
                        AttributeValue::Integer(val) => assert_eq!(val, answer),
                        _ => panic!("Unexpected attribute type"),
                    };
                    match arr[2] {
                        AttributeValue::Float(val) => assert_eq!(val, pi),
                        _ => panic!("Unexpected attribute type"),
                    }
                },
                _ => panic!("Unexpected attribute type"),
            };
        }

        {
            // Test color object attribute
            let color_red_attrval = metadata.first_attrval_by_path( &["color", "red"] );
            match color_red_attrval.unwrap() {
                AttributeValue::Integer(val) => assert_eq!(val, 90),
                _ => panic!("Unexpected attribute type"),
            };

            let color_green_attrval = metadata.first_attrval_by_path( &["color", "green"] );
            match color_green_attrval.unwrap() {
                AttributeValue::Integer(val) => assert_eq!(val, 60),
                _ => panic!("Unexpected attribute type"),
            };

            let color_purple_attrval = metadata.first_attrval_by_path( &["color", "purple"] );
            assert!( color_purple_attrval.is_none() );
        }
    }
}
