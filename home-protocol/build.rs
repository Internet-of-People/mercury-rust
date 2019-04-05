fn main() {
    capnpc::CompilerCommand::new()
        .edition(capnpc::RustEdition::Rust2018)
        .file("protocol/mercury.capnp")
        .run()
        .unwrap();
}
