use failure::Fallible;
use serde_derive::{Deserialize, Serialize};

use crate::model::*;



#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub(crate) struct Envelope
{
    pub(crate) target: String,

    #[serde(serialize_with = "serialize_byte_vec")]
    #[serde(deserialize_with = "deserialize_byte_vec")]
    pub(crate) payload: Vec<u8>,
}

impl Envelope
{
    pub(crate) fn new(target: &str, payload: Vec<u8>) -> Self
        { Self{ target: target.to_owned(), payload } }

    pub(crate) fn from<T: serde::Serialize>(target: &str, payload: T) -> Fallible<Self>
    {
        let payload_bin = rmp_serde::to_vec_named(&payload)?;
        Ok( Self::new(target, payload_bin) )
    }
}



type MessageId = u32;

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct Request<T>
{
    rid: u32,
    method: String,
    params: T,
}

impl<T> Request<T> where T: serde::Serialize
{
    pub(crate) fn new(rid: u32, method: &str, params: T) -> Self
        { Self{ rid, method: method.to_owned(), params } }
}



#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct Response
{
    pub rid: u32,
    pub code: u32,
    pub description: Option<String>,
    pub reply: rmpv::Value,
}

impl Response
{
    pub fn new(rid: u32, code: u32, description: Option<String>, reply: rmpv::Value) -> Self
        { Self{ rid, code, description, reply } }
}



#[derive(Clone, Debug, Deserialize, Eq, Hash, PartialEq, PartialOrd, Serialize)]
pub(crate) struct AddNodeParams
{
    pub(crate) id: ProfileId,
}

#[derive(Clone, Debug, Deserialize, Eq, Hash, PartialEq, PartialOrd, Serialize)]
pub(crate) struct AddEdgeParams
{
    pub(crate) source: ProfileId,
    pub(crate) target: ProfileId,
}

#[derive(Clone, Debug, Deserialize, Eq, Hash, PartialEq, PartialOrd, Serialize)]
pub(crate) struct AddEdgeReply
{
    pub(crate) id: LinkId,
//    pub(crate) source: ProfileId,
//    pub(crate) target: ProfileId,
}

#[derive(Clone, Debug, Deserialize, Eq, Hash, PartialEq, PartialOrd, Serialize)]
pub(crate) struct SetAttributeParams
{
    pub(crate) key: AttributeId,
    pub(crate) value: AttributeValue,
}

#[derive(Clone, Debug, Deserialize, Eq, Hash, PartialEq, PartialOrd, Serialize)]
pub(crate) struct ClearAttributeParams
{
    pub(crate) key: AttributeId,
}



#[test]
fn test_serialization_concept()
{
    let original_envelope = {
        let params = AddEdgeParams{ source: ProfileId{id: vec![2]}, target: ProfileId{id: vec![42]} };
        let request = Request::new(1, "add_edge", params);
        // println!("request: {:#?}", request);
        Envelope::from("target", request)
            .expect("Failed to build envelope from request")
    };

    // println!("envelope: {:?}", original_envelope);
    let envelope_bytes = rmp_serde::encode::to_vec_named(&original_envelope)
        .expect("Failed to serialize envelope");

//    use std::io::Cursor;
//    let mut read_cursor = Cursor::new(&envelope_bytes);
    let read_envelope: Envelope = rmp_serde::decode::from_slice(&envelope_bytes)
        .expect("Failed to parse envelope");
    assert_eq!(read_envelope, original_envelope);
    // debug!("envelope: {:?}", read_envelope);
}



//fn value_serialization_experiment()
//{
//    use std::io::Cursor;
//    let mut buffer = vec![0u8; 64];
//    let mut write_cursor = Cursor::new(&mut buffer);
//
//    use rmpv::Value;
//    let mut fields = Vec::new();
//    fields.push( (Value::String( rmpv::Utf8String::from("egy") ), Value::Integer( rmpv::Integer::from(1) ) ) );
//    let write_val = Value::Map( fields.clone() );
//    rmpv::encode::write_value(&mut write_cursor, &write_val).unwrap();
//
//    let mut read_cursor = Cursor::new(&buffer);
//    let read_val = rmpv::decode::read_value(&mut read_cursor).unwrap();
//    assert_eq!(read_val, write_val);
//    match read_val {
//        Value::Map(map) => { assert_eq!(map, fields) },
//        _ => { assert!(false) }
//    }
//}
//
//
//fn manual_messagepack_parsing_experiment()
//{
//    use std::io::Cursor;
//    let mut buffer = vec![0u8; 64];
//    let mut cursor = Cursor::new(&mut buffer);
//
//    use rmp::decode;
//    fn read_str<R: std::io::Read>(mut r: R) -> Option<String>
//    {
//        let str_length = decode::read_str_len(&mut r).ok()?;
//        let mut content = Vec::new();
//        content.resize(str_length as usize, 0);
//        decode::read_str(&mut r, &mut content).ok()?;
//        Some( String::from_utf8(content).ok()? )
//    }
//
//    let map_length = decode::read_map_len(&mut cursor).unwrap();
//    let name = read_str(&mut cursor).unwrap();
//    match name.as_ref() {
//        "target" => { let target = read_str(&mut cursor).unwrap(); },
//        "payload" => { let val = rmpv::decode::read_value(&mut cursor); },
//        _ => {}
//    }
//}
