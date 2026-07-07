fn main() {
    tauri_build::build();

    // Add rpath for Swift runtime libraries needed by mado & cgevents at runtime.
    // macOS 15+ ships Swift dylibs under /usr/lib/swift/ (dyld shared cache).
    // cgevents' build.rs sets -rpath /usr/lib/swift but that only applies to
    // the library itself, not the final binary — so we set it here.
    println!("cargo:rustc-link-arg-bins=-Wl,-rpath,/usr/lib/swift");
}
