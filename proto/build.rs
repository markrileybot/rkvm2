use std::env;
use std::io::Result;
use std::path::PathBuf;

use prost_wkt_build::{FileDescriptorSet, Message};

use version_rs::version;

extern crate prost_wkt_build;
extern crate version_rs;

fn main() -> Result<()> {
    let version_string = version(
        env!("CARGO_PKG_NAME"),
        env!("CARGO_PKG_VERSION"),
        env!("CARGO_MANIFEST_DIR"),
    );
    println!(
        "cargo:rustc-env=DTX_PROTO_VERSION_STRING={}",
        version_string
    );

    let out = PathBuf::from(env::var("OUT_DIR").unwrap());
    let descriptor_file = out.join("descriptors.bin");
    let mut prost_build = prost_build::Config::new();
    prost_build
        .type_attribute(".", "#[derive(serde::Serialize,serde::Deserialize)]")
        .type_attribute_with_filter(".", "#[derive(FromPrimitive,ToPrimitive)]", prost_build::TypeSelector::ProtobufEnum)
        .type_attribute_with_filter(".", "#[serde(default)]", prost_build::TypeSelector::ProtobufMessage)
        .extern_path(".google.protobuf.Any", "::prost_wkt_types::Any")
        .extern_path(".google.protobuf.Timestamp", "::prost_wkt_types::Timestamp")
        .extern_path(".google.protobuf.Value", "::prost_wkt_types::Value")
        .file_descriptor_set_path(&descriptor_file)
        .compile_protos(&["src/messages.proto"], &["src/"])?;

    let descriptor_bytes = std::fs::read(descriptor_file).unwrap();

    let descriptor = FileDescriptorSet::decode(&descriptor_bytes[..]).unwrap();

    prost_wkt_build::add_serde(out, descriptor);
    Ok(())
}
