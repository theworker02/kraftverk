//! Emit SPIR-V for the GPU compute kernel at build time (naga, no system glslc).

fn main() {
    if std::env::var("CARGO_FEATURE_GPU").is_err() {
        return;
    }
    let wgsl = r#"
@group(0) @binding(0) var<storage, read_write> data: array<u32>;

@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
    let i = gid.x;
    if (i >= arrayLength(&data)) { return; }
    var x = data[i];
    x = x * 1664525u + 1013904223u;
    x = x ^ (i * 2654435761u);
    data[i] = x;
}
"#;
    let module = naga::front::wgsl::parse_str(wgsl).expect("parse wgsl");
    let info = naga::valid::Validator::new(
        naga::valid::ValidationFlags::all(),
        naga::valid::Capabilities::all(),
    )
    .validate(&module)
    .expect("validate wgsl");
    let spv =
        naga::back::spv::write_vec(&module, &info, &naga::back::spv::Options::default(), None)
            .expect("emit spirv");
    let out = std::path::Path::new(&std::env::var("OUT_DIR").unwrap()).join("compute.spv");
    let bytes: &[u8] = bytemuck::cast_slice(&spv);
    std::fs::write(&out, bytes).expect("write spirv");
    println!("cargo:rerun-if-changed=build.rs");
}
