use cmake;

fn main() {
    let hoppet = cmake::Config::new("hoppet")
        .define("HOPPET_BUILD_EXAMPLES", "OFF")
        .define("HOPPET_ENABLE_TESTING", "OFF")
        .define("HOPPET_BUILD_BENCHMARK", "OFF")
        .env("CMAKE_Fortran_FLAGS", "-frecursive -fcheck=no-recursive")
        .profile("Release")
        .build();
    println!(
        "cargo:rustc-link-search=native={}",
        hoppet.join("lib").display()
    );
    println!(
        "cargo:rustc-link-search=native={}",
        hoppet.join("lib64").display()
    );
    println!("cargo:rustc-link-lib=static=hoppet");
    println!("cargo:rustc-link-lib=dylib=gfortran");
    println!("cargo:rustc-link-lib=dylib=stdc++");
}
