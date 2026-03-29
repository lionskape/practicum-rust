fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_build::compile_protos("proto/blog.proto")?;
    println!("cargo:rerun-if-changed=proto/blog.proto");
    Ok(())
}
