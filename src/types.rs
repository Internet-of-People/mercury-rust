#[derive(Clone, Debug, Eq, Hash, PartialEq, PartialOrd)]
pub struct ProfileId( pub Vec<u8> );

pub type LinkId = ProfileId;
pub type AttributeId = String;
pub type AttributeValue = String;



impl<'a> From<&'a ProfileId> for String
{
    fn from(src: &'a ProfileId) -> Self
        { ::multibase::encode(::multibase::Base::Base64url, &src.0) }
}

impl<'a> From<ProfileId> for String
{
    fn from(src: ProfileId) -> Self
        { Self::from(&src) }
}

impl std::fmt::Display for ProfileId {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", String::from(self))
    }
}

impl std::str::FromStr for ProfileId {
    type Err = multibase::Error;
    fn from_str(src: &str) -> Result<Self, Self::Err> {
        let (_base, binary) = ::multibase::decode(src)?;
        Ok( ProfileId(binary) )
    }
}
