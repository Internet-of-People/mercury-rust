extern crate capnpc;


fn main()
{
    ::capnpc::CompilerCommand::new()
        .file("protocol/mercury.capnp")
        .run().unwrap();
}
