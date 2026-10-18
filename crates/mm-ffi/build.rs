fn main() {
    // flutter_rust_bridge's attribute macros expand through this internal cfg.
    println!("cargo:rustc-check-cfg=cfg(frb_expand)");
}
