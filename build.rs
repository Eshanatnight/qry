fn main() {
    // DuckDB's AdditionalLockInfo uses the Windows Restart Manager API,
    // which requires linking against rstrtmgr.lib.
    #[cfg(target_os = "windows")]
    println!("cargo:rustc-link-lib=rstrtmgr");
}
