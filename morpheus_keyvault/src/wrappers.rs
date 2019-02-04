pub trait BaseEncodable : AsRef<[u8]> {
    fn get_base_code(&self) -> char;
}

impl<'a, T: BaseEncodable + ?Sized> BaseEncodable for &'a T {
    fn get_base_code(&self) -> char {
        (**self).get_base_code()
    }
}

impl<'a> From<&'a BaseEncodable> for String
{
    fn from(src: &'a BaseEncodable) -> Self {
        ::multibase::encode(::multibase::Base::from_code(src.get_base_code()).unwrap(), src)
    }
}

impl std::fmt::Display for BaseEncodable {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", String::from(self))
    }
}

trait AbstractFactory<T: Sized> {
    fn create(binary: Vec<u8>) -> T;
}

struct ConcreteType {
    pub data: Vec<u8>,
}

struct ConcreteFactory {}

impl AbstractFactory<ConcreteType> for ConcreteFactory {
    fn create(binary: Vec<u8>) -> ConcreteType {
        ConcreteType { data: binary }
    }
}

struct SomeAlgorithm {}

impl SomeAlgorithm {
    fn algo<T, F: AbstractFactory<T>>(f: F) -> T {
        let bin = vec![0u8];
        F::create(bin)
    }
}


#[test]
fn test_factory() {
    let i = SomeAlgorithm::algo(ConcreteFactory {});
    assert_eq!(i.data.len(), 1);
}

// struct EncodableData {
//     prefix: char,
//     base: ::multibase::Base,
//     binary: Vec<u8>,
// }

// impl std::str::FromStr for EncodableData {
//     type Err = failure::Error;
//     fn from_str(src: &str) -> Result<Self, Self::Err> {
//         if src.is_empty() {
//             bail!("Cannot parse EncodableData from empty string")
//         }

//         let mut chars = src.chars();
//         let prefix = chars.next().unwrap();
//         let rest = chars.as_str();

//         let (base, binary) = ::multibase::decode(rest)?;
//         Ok( EncodableData { prefix, base, binary } )
//     }
// }
