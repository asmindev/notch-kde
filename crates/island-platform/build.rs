fn main() {
    println!("cargo:rustc-link-lib=wayland-client");

    cc::Build::new()
        .file("c/plasma_window_management.c")
        .file("c/org_kde_plasma_virtual_desktop.c")
        .file("c/dodge_native.c")
        .include("c")
        .compile("dodge_native");
}
