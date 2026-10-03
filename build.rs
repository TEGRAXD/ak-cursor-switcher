// Build script (build.rs) executed by Cargo prior to compiling the crate.
// Links the Common Controls v6 manifest into the executable for modern visual styles,
// rounded controls, and hardware-accelerated smooth scrolling on Windows.
fn main() {
    #[cfg(target_os = "windows")]
    {
        // /MANIFEST:EMBED tells the MSVC linker to embed the generated manifest into the PE resource section
        println!("cargo:rustc-link-arg=/MANIFEST:EMBED");
        // /MANIFESTDEPENDENCY declares comctl32.dll version 6.0 as a dependency
        println!(
            "cargo:rustc-link-arg=/MANIFESTDEPENDENCY:type='win32' name='Microsoft.Windows.Common-Controls' version='6.0.0.0' processorArchitecture='*' publicKeyToken='6595b64144ccf1df' language='*'"
        );
    }
}
