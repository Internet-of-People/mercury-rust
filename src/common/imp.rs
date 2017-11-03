use multihash;

use multibase;
use serde;
use serde_json;

use common::*;



pub struct MultiHasher
{
    hash_algorithm: multihash::Hash,
}

impl MultiHasher
{
    pub fn new(hash_algorithm: multihash::Hash) -> Self
        { MultiHasher{hash_algorithm: hash_algorithm} }

    fn to_hasher_error(error: multihash::Error) -> HashError
    {
        match error {
            multihash::Error::BadInputLength    => HashError::BadInputLength,
            multihash::Error::UnkownCode        => HashError::UnknownCode,
            multihash::Error::UnsupportedType   => HashError::UnsupportedType,
        }
    }

    fn get_hash_bytes(&self, data: &Vec<u8>) -> Result<Vec<u8>, HashError>
    {
        multihash::encode(self.hash_algorithm, data)
            .map_err(MultiHasher::to_hasher_error)
    }

//    fn get_hash_string(&self, data: &Vec<u8>) -> Result<String, HashError>
//    {
//        self.get_hash_bytes(&data)
//            .map( |bytes| multibase::encode(multibase::Base64, &bytes) )
//    }
}

//impl Hasher<Vec<u8>, Vec<u8>> for MultiHasher
//{
//    fn get_hash(&self, data: &Vec<u8>) -> Result<Vec<u8>, HashError>
//        { self.get_hash_bytes(&data) }
//
//    fn validate(&self, data: &Vec<u8>, expected_hash: &Vec<u8>) -> Result<bool, HashError>
//    {
//        // TODO should we do this here or just drop this step and check hash equality?
//        let decode_result = decode(expected_hash)
//            .map_err(MultiHasher::to_hasher_error)?;
//        if decode_result.alg != self.hash_algorithm
//            { return Err(HashError::UnsupportedType); }
//
//        let calculated_hash = self.get_hash_bytes(&data)?;
//        Ok(*expected_hash == calculated_hash)
//    }
//}

impl Hasher<Vec<u8>, Vec<u8>> for MultiHasher
{
    fn get_hash(&self, data: &Vec<u8>) -> Result<Vec<u8>, HashError>
        { self.get_hash_bytes(&data) }

    fn validate(&self, data: &Vec<u8>, expected_hash: &Vec<u8>) -> Result<bool, HashError>
    {
//        // TODO should we do this here or just drop this step and check hash equality?
//        let decode_result = decode(expected_hash)
//            .map_err(MultiHasher::to_hasher_error)?;
//        if decode_result.alg != self.hash_algorithm
//            { return Err(HashError::UnsupportedType); }

        let calculated_hash = self.get_hash_bytes(&data)?;
        Ok(*expected_hash == calculated_hash)
    }
}




// TODO this struct should be independent of the serialization format (e.g. JSON):
//      Maybe should contain Box<serde::ser::De/Serializer> data members
pub struct SerdeJsonSerializer;

impl SerdeJsonSerializer
{
    fn to_serializer_error(error: serde_json::Error) -> SerializerError {
        SerializerError::SerializationError( Box::new(error) )
    }
}

impl<ObjectType> Serializer<ObjectType, Vec<u8>> for SerdeJsonSerializer
    where ObjectType: serde::Serialize + serde::de::DeserializeOwned
{
    fn serialize(&self, object: &ObjectType) -> Result<Vec<u8>, SerializerError>
    {
        serde_json::to_string(&object)
            .map( |str| str.into_bytes() )
            .map_err(SerdeJsonSerializer::to_serializer_error)
    }

    fn deserialize(&self, serialized_object: &Vec<u8>) -> Result<ObjectType, SerializerError>
    {
        let json_string = String::from_utf8(serialized_object.clone() )
            .map_err(|e| SerializerError::DeserializationError( Box::new(e) ) )?;
        serde_json::from_str(& json_string)
            .map_err(SerdeJsonSerializer::to_serializer_error)
    }
}



pub struct MultiBaseStringCoder
{
    base_algorithm: multibase::Base,
}

impl MultiBaseStringCoder
{
    pub fn new(base_algorithm: multibase::Base) -> Self
    { Self{base_algorithm: base_algorithm} }
}

impl StringCoder<Vec<u8>> for MultiBaseStringCoder
{
    fn encode(&self, hash_bytes: &Vec<u8>) -> Result<String, StringCoderError>
    {
        let hash_str = multibase::encode(self.base_algorithm, &hash_bytes);
        Ok(hash_str)
    }

    fn decode(&self, hash_str: &str) -> Result<Vec<u8>, StringCoderError>
    {
        multibase::decode(&hash_str)
            .map( |(_,bytes)| bytes )
            .map_err( |e| StringCoderError::Other( Box::new(e) ) )
    }
}



#[cfg(test)]
mod tests
{
    use super::*;


    #[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
    struct Person
    {
        name:  String,
        phone: String,
        age:   u16,
    }


    #[test]
    fn test_serializer()
    {
        let serializer = SerdeJsonSerializer;
        let orig_obj = Person{ name: "Aladar".to_string(), phone: "+36202020202".to_string(), age: 28 };
        let ser_obj = serializer.serialize(&orig_obj);
        assert!( ser_obj.is_ok() );
        let deser_res = serializer.deserialize( &ser_obj.unwrap() );
        assert!( deser_res.is_ok() );
        assert_eq!( orig_obj, deser_res.unwrap() );
    }

    #[test]
    fn test_hasher()
    {
        let ser_obj = vec![1, 2, 3, 4, 5, 6, 7, 8, 9];
        let hasher = MultiHasher::new(multihash::Hash::Keccak256);
        let hash = hasher.get_hash(&ser_obj);
        assert!( hash.is_ok() );
        let valid = hasher.validate( &ser_obj, &hash.unwrap() );
        assert!( valid.is_ok() );
    }


    #[test]
    fn test_string_coder()
    {
        let hash_bytes = vec![1, 2, 3, 4, 5, 6, 7, 8, 9];
        let coder = MultiBaseStringCoder::new(multibase::Base64);
        let hash_str = coder.encode(&hash_bytes);
        assert!( hash_str.is_ok() );
        let decode_res = coder.decode( &hash_str.unwrap() );
        assert!( decode_res.is_ok() );
        assert_eq!( hash_bytes, decode_res.unwrap() );
    }
}
