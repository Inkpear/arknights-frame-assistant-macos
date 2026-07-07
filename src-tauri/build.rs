fn main() {
    tauri_build::build();

    // Add rpath for Swift runtime libraries needed by mado & cgevents at runtime.
    println!("cargo:rustc-link-arg=-Wl,-rpath,/usr/lib/swift");
}
