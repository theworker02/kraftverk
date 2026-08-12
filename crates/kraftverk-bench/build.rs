//! Emit SPIR-V for GPU compute kernels at build time (naga, no system glslc).

fn write_spv(name: &str, wgsl: &str) {
    let module = naga::front::wgsl::parse_str(wgsl).unwrap_or_else(|e| panic!("parse {name}: {e}"));
    let info = naga::valid::Validator::new(
        naga::valid::ValidationFlags::all(),
        naga::valid::Capabilities::all(),
    )
    .validate(&module)
    .unwrap_or_else(|e| panic!("validate {name}: {e}"));
    let spv =
        naga::back::spv::write_vec(&module, &info, &naga::back::spv::Options::default(), None)
            .unwrap_or_else(|e| panic!("emit {name}: {e}"));
    let out = std::path::Path::new(&std::env::var("OUT_DIR").unwrap()).join(name);
    let bytes: &[u8] = bytemuck::cast_slice(&spv);
    std::fs::write(&out, bytes).unwrap_or_else(|e| panic!("write {name}: {e}"));
}

fn main() {
    if std::env::var("CARGO_FEATURE_GPU").is_err() {
        return;
    }
    write_spv(
        "compute.spv",
        r#"
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
"#,
    );
    write_spv(
        "reduce.spv",
        r#"
@group(0) @binding(0) var<storage, read_write> data: array<u32>;

@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
    let i = gid.x;
    let n = arrayLength(&data);
    let half = n / 2u;
    if (i >= half) { return; }
    data[i] = data[i] ^ data[i + half];
}
"#,
    );
    println!("cargo:rerun-if-changed=build.rs");
}
