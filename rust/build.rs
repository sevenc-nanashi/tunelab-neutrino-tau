fn main() {
    csbindgen::Builder::default()
        .input_extern_file("src/lib.rs")
        .csharp_namespace("NeutrinoTau.Native")
        .csharp_class_name("NativeMethods")
        .csharp_dll_name("neutrino_tau_native")
        .generate_csharp_file("../src/Native/Generated/NativeMethods.g.cs")
        .expect("Unable to generate C# bindings.");

    println!("cargo:rerun-if-changed=src/lib.rs");
}
