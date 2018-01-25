use std::collections::HashMap;
use std::hash::Hash;
use std::rc::Rc;

use futures::prelude::*;
use futures::future;
use futures_state_stream::StateStream;
use ipfs_api;
use multibase;
use tokio_core::reactor;
use tokio_postgres;

use async::*;
use common::HashWebLink;
use format::*;



pub struct HashWeb<ObjectType>
{
    hashspaces: HashMap< HashSpaceId, Box< HashSpace<ObjectType, String> > >,
    default:    HashSpaceId,
}


impl<ObjectType: 'static> HashWeb<ObjectType>
{
    pub fn new(hashspaces: HashMap< HashSpaceId, Box< HashSpace<ObjectType, String> > >,
               default: HashSpaceId) -> Self
        { HashWeb { hashspaces: hashspaces, default: default } }


    // Expected hashlink format: hashspaceId/hash
    pub fn resolve_hashlink(&self, hashlink_str: &str)
        -> Box< Future<Item=ObjectType, Error=AddressResolutionError> >
    {
        // Ignore starting slash
        let address = if hashlink_str.starts_with('/') { &hashlink_str[1..] } else { hashlink_str };

        // Split hashspaceId and hash parts
        let slash_pos = address.find('/').unwrap_or( address.len() );
        let (hashspace_id, slashed_hash) = hashlink_str.split_at(slash_pos);
        let hash = &slashed_hash[1..]; // Ignore starting slash

        // Perform link resolution
        let hashlink = HashWebLink::new( &hashspace_id.to_string(), hash );
        let resolved_data_fut = self.resolve(&hashlink)
            .map_err( |e| AddressResolutionError::HashSpaceError(e) );
        Box::new(resolved_data_fut)
    }
}


impl<ObjectType>
HashSpace<ObjectType, HashWebLink>
for HashWeb<ObjectType>
where ObjectType: 'static
{
    fn store(&mut self, object: ObjectType)
         -> Box< Future<Item=HashWebLink, Error=HashSpaceError> >
    {
        let mut hashspace_res = self.hashspaces.get_mut(&self.default)
            .ok_or( HashSpaceError::UnsupportedHashSpace( self.default.to_owned() ) );;
        let hashspace = match hashspace_res {
            Ok(ref mut space) => space,
            Err(e) => return Box::new( future::err(e) ),
        };
        let default_hashspace_clone = self.default.clone();
        let result = hashspace.store(object)
            .map( move |hash| HashWebLink::new(&default_hashspace_clone, &hash) );
        Box::new(result)
    }


    fn resolve(&self, link: &HashWebLink)
        -> Box< Future<Item = ObjectType, Error = HashSpaceError> >
    {
        let hashspace_res = self.hashspaces.get( link.hashspace() )
            .ok_or( HashSpaceError::UnsupportedHashSpace( link.hashspace().to_owned() ) );
        let hashspace = match hashspace_res {
            Ok(space) => space,
            Err(e) => return Box::new( future::err(e) ),
        };
        let data = hashspace.resolve( &link.hash().to_owned() );
        Box::new(data)
    }


    fn validate(&self, object: &ObjectType, link: &HashWebLink)
        -> Box< Future<Item=bool, Error=HashSpaceError> >
    {
        let hashspace_res = self.hashspaces.get( link.hashspace() )
            .ok_or( HashSpaceError::UnsupportedHashSpace( link.hashspace().to_owned() ) );
        let hashspace = match hashspace_res {
            Ok(ref space) => space,
            Err(e) => return Box::new( future::err(e) ),
        };
        // TODO to_string() is unnecessary below, find out how to transform signatures so as it's not needed
        let result = hashspace.validate( object, &link.hash().to_string() );
        Box::new(result)
    }
}



pub struct AddressResolver
{
    hashweb:            Rc< HashWeb< Vec<u8> > >,
    format_registry:    Rc<FormatRegistry>,
}


type FutureBlob = Box< Future<Item=Vec<u8>, Error=AddressResolutionError> >;

impl AddressResolver
{
    pub fn new(formats: FormatRegistry, hashweb: HashWeb< Vec<u8> >) -> Self
        { Self{ format_registry: Rc::new(formats), hashweb: Rc::new(hashweb) } }


    // Address format expected by the parser, optionally with a starting /
    // hashspaceId/hash#formatId@path/to/hashlink/attribute&formatId@path/to/another/attribute
    //  ^^ hashlink ^^ # ^^^^^ (link) attr specifier ^^^^^ & ^^^^^^ attribute specifier ^^^^^
    pub fn resolve_blob<'s,'a>(&'s self, address: &'a str) -> FutureBlob
    {
        // Separate starting hashlink part from attributes
        let attr_separ_idx = address.find('#').unwrap_or( address.len() );
        let (hashlink_str, hashed_attr_specs_str) = address.split_at(attr_separ_idx);

        // Resolve hashlink into binary blob
        let mut blob_fut: FutureBlob = Box::new( self.hashweb.resolve_hashlink(hashlink_str) );

        // Separate (possibly many) attribute references
        let attr_specs_str = &hashed_attr_specs_str[1..];
        let attribute_specs: Vec<&str> = attr_specs_str.split('&').collect();
        for attr_spec in attribute_specs
        {
            let formats_clone = self.format_registry.clone();
            let hashweb_clone = self.hashweb.clone();
            let attrspec_clone = attr_spec.to_owned();
            // Perform resolution of next attribute apecifier
            blob_fut = Box::new( blob_fut.and_then( move |blob|
            {
                // Parse blob and query attribute path as hashweblink
                let hashlink_res = formats_clone.resolve_attr_link(&blob, &attrspec_clone);
                match hashlink_res {
                    Err(e) => Box::new( future::err(e) ) as FutureBlob,
                    Ok(hashlink) => {
                        // Resolve hashweblink as blob
                        let resolved_link_fut = hashweb_clone.resolve(&hashlink)
                            .map_err( |e| AddressResolutionError::HashSpaceError(e) );
                        Box::new(resolved_link_fut) as FutureBlob
                    }
                }

            } ) );
        }
        blob_fut
    }
}



pub struct Ipfs
{
    client: ipfs_api::IpfsClient,
}

impl Ipfs
{
    pub fn new(client: ipfs_api::IpfsClient) -> Self
        { Self{ client: client} }
}

impl HashSpace<Vec<u8>, String> for Ipfs
{
    fn store(&mut self, object: Vec<u8>)
        -> Box< Future<Item=String, Error=HashSpaceError> >
    {
        // TODO maybe we should also pin the object after adding
        let data = ::std::io::Cursor::new(object);
        let add_fut = self.client.add(data)
            .map( |resp| resp.hash )
            // TODO error should be mapped to something more descriptive than Other
            .map_err( |e| HashSpaceError::Other( Box::new(e) ) );
        Box::new(add_fut)
    }

    fn resolve(&self, hash: &String)
        -> Box< Future<Item=Vec<u8>, Error=HashSpaceError> >
    {
        let cat_fut = self.client.cat(hash).concat2()
            .map( |chunk| chunk.to_vec() )
            // TODO error should be mapped to something more descriptive than Other
            .map_err( |e| HashSpaceError::Other( Box::new(e) ) );
        Box::new(cat_fut)
    }

    fn validate(&self, _object: &Vec<u8>, hash: &String)
        -> Box< Future<Item=bool, Error=HashSpaceError> >
    {
        let val_fut = self.resolve(hash)
            .map( |_bytes| true );
        Box::new(val_fut)
    }
}



pub struct InMemoryStore<KeyType, ValueType>
{
    map: HashMap<KeyType, ValueType>,
}

impl<KeyType, ValueType> InMemoryStore<KeyType, ValueType>
    where KeyType: Eq + Hash
{
    pub fn new() -> Self { InMemoryStore{ map: HashMap::new() } }
}

impl<KeyType, ValueType>
KeyValueStore<KeyType, ValueType>
for InMemoryStore<KeyType, ValueType>
    where KeyType: Eq + Hash,
          ValueType: Clone + 'static
{
    fn store(&mut self, key: KeyType, object: ValueType)
        -> Box< Future<Item=(), Error=StorageError> >
    {
        self.map.insert(key, object );
        Box::new( future::ok(() ) )
    }

    fn lookup(&self, key: KeyType)
        -> Box< Future<Item=ValueType, Error=StorageError> >
    {
        let result = match self.map.get(&key) {
            Some(val) => future::ok( val.to_owned() ),
            None      => future::err(StorageError::InvalidKey),
        };
        Box::new(result)
    }
}



pub struct PostgresStore
{
    reactor_handle: reactor::Handle,
    postgres_url:   String,
    table:          String,
    key_col:        String,
    value_col:      String,
}

impl PostgresStore
{
    pub fn new(reactor_handle: &reactor::Handle, postgres_url: &str,
               table: &str, key_col: &str, value_col: &str) -> Self
    {
        Self{ reactor_handle:   reactor_handle.clone(),
              postgres_url:     postgres_url.to_string(),
              table:            table.to_string(),
              key_col:          key_col.to_string(),
              value_col:        value_col.to_string(), }
    }

    fn prepare(&self, sql_statement: String)
        -> Box< Future<Item=(tokio_postgres::stmt::Statement,tokio_postgres::Connection), Error=(tokio_postgres::Error,tokio_postgres::Connection)> >
    {
        let result = tokio_postgres::Connection::connect( self.postgres_url.as_str(),
                tokio_postgres::TlsMode::None, &self.reactor_handle)
            .then( move |conn| {
                conn.expect("Connection to database failed")
                    .prepare(&sql_statement)
            } );
        Box::new(result)
    }
}

impl KeyValueStore<Vec<u8>, Vec<u8>> for PostgresStore
{
    fn store(&mut self, key: Vec<u8>, value: Vec<u8>)
        -> Box< Future<Item=(), Error=StorageError> >
    {
        let key_str = multibase::encode(multibase::Base64, &key);
        let sql = format!("INSERT INTO {0} ({1}, {2}) VALUES ($1, $2)",
            self.table, self.key_col, self.value_col);
        let result = self.prepare(sql)
            .and_then( move |(stmt, conn)| {
                conn.execute(&stmt, &[&key_str, &value])
            } )
            .map_err( |(e,_conn)| StorageError::Other( Box::new(e) ) )
            .map( |_exec_res| () );
        Box::new(result)
    }

    fn lookup(&self, key: Vec<u8>)
        -> Box< Future<Item=Vec<u8>, Error=StorageError> >
    {
        let key_str = multibase::encode(multibase::Base64, &key);
        let sql = format!("SELECT {1}, {2} FROM {0} WHERE {1}=$1",
            self.table, self.key_col, self.value_col);
        let result = self.prepare( sql.to_string() )
            .and_then( move |(stmt, conn)|
                conn.query(&stmt, &[&key_str])
                    .map( |row| {
                        let value: Vec<u8> = row.get(1);
                        value
                    } )
                    .collect()
                    // TODO reducing resulsts by concatenating provides bad results
                    //      if multiple rows are found in the result set
                    .map( |(vec,_state)| vec.concat() )
            )
            .map_err( |(e,_conn)| StorageError::Other( Box::new(e) ) );

        Box::new(result)
    }
}



#[cfg(test)]
mod tests
{
    use multihash;
    use tokio_core::reactor;

    use common::imp::*;
    use super::*;


    #[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
    struct Person
    {
        name:  String,
        phone: String,
        age:   u16,
    }


    #[test]
    fn test_inmemory_storage()
    {
        // NOTE this works without a tokio::reactor::Core only because
        //      the storage always returns an already completed future::ok/err result
        let object = Person{ name: "Aladar".to_string(), phone: "+36202020202".to_string(), age: 28 };
        let hash = "key".to_string();
        let mut storage: InMemoryStore<String,Person> = InMemoryStore::new();
        let store_res = storage.store( hash.clone(), object.clone() ).wait();
        assert!( store_res.is_ok() );
        let lookup_res = storage.lookup(hash).wait();
        assert!( lookup_res.is_ok() );
        assert_eq!( lookup_res.unwrap(), object );
    }


    #[test]
    fn test_postgres_storage()
    {
        let mut reactor = reactor::Core::new()
            .expect("Failed to initialize the reactor event loop");

        // This URL should be assembled from the gitlab-ci.yml
        let postgres_url = "postgresql://testuser:testpass@postgres/testdb";
        let mut storage = PostgresStore::new( &reactor.handle(),
            postgres_url, "storagetest", "key", "data");

        let key = b"key".to_vec();
        let value = b"value".to_vec();
        let store_future = storage.store( key.clone(), value.clone() );
        let store_res = reactor.run(store_future);
        assert!( store_res.is_ok(), "store failed with {:?}", store_res );

        let lookup_future = storage.lookup(key);
        let lookup_res = reactor.run(lookup_future);
        assert!( lookup_res.is_ok(), "lookup failed with {:?}", lookup_res );
        assert_eq!( lookup_res.unwrap(), value );
    }


    #[test]
    fn test_hashspace()
    {
        // NOTE this works without a tokio::reactor::Core only because
        //      all plugins always return an already completed ok/err result
        let store: InMemoryStore<Vec<u8>, Vec<u8>> = InMemoryStore::new();
        let mut hashspace: ModularHashSpace<Vec<u8>, Vec<u8>, String> = ModularHashSpace::new(
//            Rc::new( SerdeJsonSerializer{} ),
            Rc::new( MultiHasher::new(multihash::Hash::Keccak512) ),
            Box::new(store),
            Box::new( MultiBaseHashCoder::new(multibase::Base64) ) );

//        let object = Person{ name: "Aladar".to_string(), phone: "+36202020202".to_string(), age: 28 };
        let object = b"What do you get if you multiply six by nine?".to_vec();
        let store_res = hashspace.store( object.clone() ).wait();
        assert!( store_res.is_ok() );
        let hash = store_res.unwrap();
        let lookup_res = hashspace.resolve(&hash).wait();
        assert!( lookup_res.is_ok() );
        assert_eq!( lookup_res.unwrap(), object );
        let validate_res = hashspace.validate(&object, &hash).wait();
        assert!( validate_res.is_ok() );
        assert!( validate_res.unwrap() );
    }


    #[test]
    fn test_hashweb()
    {
        let cache_store: InMemoryStore<Vec<u8>, Vec<u8>> = InMemoryStore::new();
        let cache_space: ModularHashSpace<Vec<u8>, Vec<u8>, String> = ModularHashSpace::new(
//            Rc::new( IdentitySerializer{} ),
            Rc::new( MultiHasher::new(multihash::Hash::Keccak512) ),
            Box::new(cache_store),
            Box::new( MultiBaseHashCoder::new(multibase::Base64) ) );

        let mut reactor = reactor::Core::new()
            .expect("Failed to initialize the reactor event loop");

        // This URL should be assembled from the gitlab-ci.yml
        let postgres_url = "postgresql://testuser:testpass@postgres/testdb";
        let postgres_storage = PostgresStore::new( &reactor.handle(),
            postgres_url, "storagetest", "key", "data");
        let postgres_space: ModularHashSpace< Vec<u8>, Vec<u8>, String> = ModularHashSpace::new(
//            Rc::new( IdentitySerializer{} ),
            Rc::new( MultiHasher::new(multihash::Hash::Keccak512) ),
            Box::new(postgres_storage),
            Box::new( MultiBaseHashCoder::new(multibase::Base64) ) );

        let default_space = "cache".to_owned();
        let mut spaces: HashMap< String, Box< HashSpace< Vec<u8>, String > > > = HashMap::new();
        spaces.insert( default_space.clone(), Box::new(cache_space) );
        spaces.insert( "postgres".to_owned(), Box::new(postgres_space) );
        let mut hashweb = HashWeb::new( spaces, default_space.clone() );

        let content = b"There's over a dozen netrunners Netwatch Cops would love to brain burn and Rache Bartmoss is at least two of them".to_vec();
        let link_future = hashweb.store( content.clone() );
        let link = reactor.run(link_future).unwrap();
        assert_eq!( *link.hashspace(), default_space );

        let bytes_future = hashweb.resolve(&link);
        let bytes = reactor.run(bytes_future).unwrap();
        assert_eq!( bytes, content);
    }

    #[test]
    fn test_ipfs_hashspace()
    {
        let mut reactor = reactor::Core::new()
            .expect("Failed to initialize the reactor event loop");
        let client = ipfs_api::IpfsClient::default( &reactor.handle() );
        //let client = ipfs_api::IpfsClient::new( &reactor.handle(), "localhost", 5001 ).unwrap();
        let mut ipfs = Ipfs::new(client);

        let orig_data = b"Tear down the wall!".to_vec();
        let hash_fut = ipfs.store( orig_data.clone() );
        let hash = reactor.run(hash_fut).unwrap();
println!("ipfs hash: {}", hash);
        let resolve_fut = ipfs.resolve(&hash);
        let resolved_data = reactor.run(resolve_fut).unwrap();
        assert_eq!(orig_data, resolved_data);
    }


    use meta::Attribute;
    use meta::tests::{MetaData, MetaAttr, MetaAttrVal};

    struct DummyFormatParser
    {
        link_attr: MetaAttr,
    }

    impl FormatParser for DummyFormatParser
    {
        fn parse<'b>(&self, blob: &'b [u8])
            -> Result< Box<Data + 'b>, FormatParserError >
        {
            let mut attrs = Vec::new();
            attrs.push( self.link_attr.clone() );
            Ok( Box::new( MetaData::new( blob.to_owned(), "dummy_hash".as_bytes().to_owned(), attrs ) ) )
        }
    }

    #[test]
    fn test_blob_address_resolution()
    {
        let cache_store: InMemoryStore<Vec<u8>, Vec<u8>> = InMemoryStore::new();
        let modular_cache: ModularHashSpace<Vec<u8>, Vec<u8>, String> = ModularHashSpace::new(
            Rc::new( MultiHasher::new(multihash::Hash::Keccak512) ),
            Box::new(cache_store),
            Box::new( MultiBaseHashCoder::new(multibase::Base64) ) );
        let mut cache_space = Box::new(modular_cache) as Box< HashSpace<Vec<u8>, String> >;

        let mut reactor = reactor::Core::new()
            .expect("Failed to initialize the reactor event loop");
        let myblob = Vec::from("This is my custom binary data");
        let myblob_hash_fut = cache_space.store( myblob.clone() );
        let myblob_hash = reactor.run(myblob_hash_fut).unwrap();

        let default_space = "mystore".to_owned();
        let mut spacemap = HashMap::new();
        spacemap.insert( default_space.clone(), cache_space );
        let hashweb = HashWeb::new( spacemap, default_space.clone() );

        // Test hashweb blob address resolution (without attributes), format:
        // hashspaceId/hash
        let hashlink_str = default_space.clone() + "/" + &myblob_hash;
        let resolved_fut = hashweb.resolve_hashlink(&hashlink_str);
        let resolved = reactor.run(resolved_fut).unwrap();
        assert_eq!(resolved, myblob);

        let link_attr = MetaAttr::new( "hashweblink", MetaAttrVal::LINK(
            HashWebLink::new(&default_space, &myblob_hash) ) );
        let mut attrs_vec = Vec::new();
        attrs_vec.push( link_attr.clone() );
        let container_attr = MetaAttr::new( "attributes", MetaAttrVal::OBJECT(attrs_vec) );

        let myformat = "myformat".to_owned();
        let myparser = DummyFormatParser{ link_attr: container_attr.clone() };
        let mut formats = HashMap::new();
        formats.insert( myformat.clone(), Box::new(myparser) as Box<FormatParser> );
        let registry = FormatRegistry::new(formats);

        // Test blob address resolution with link attributes, format:
        // hashspaceId/hash#formatId@path/to/hashlink/attribute&formatId@path/to/another/attribute
        let link_address = hashlink_str + "#" + &myformat + "@" +
            container_attr.name() + "/" + link_attr.name();
        let resolver = AddressResolver::new(registry, hashweb);
        let resolved_fut = resolver.resolve_blob(&link_address);

        let resolved = reactor.run(resolved_fut).unwrap();
        assert_eq!(resolved, myblob);
    }
}
