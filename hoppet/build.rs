use cmake;
use std::path::PathBuf;
use std::process::Command;

fn main() {
    let mut vendor = false;
    if let Ok(output) = Command::new("hoppet-config").arg("--version").output() {
        let version = String::from_utf8(output.stdout)
            .unwrap()
            .split(".")
            .next()
            .unwrap()
            .parse::<usize>()
            .unwrap();
        if version == 2 {
            let link_paths = String::from_utf8(Command::new("hoppet-config").arg("--libs").output().unwrap().stdout)
                .unwrap()
                .split_whitespace()
                .filter_map(|s| if s.starts_with("-L") { Some(s[2..].into()) } else { None })
                .collect::<Vec<PathBuf>>();
            for path in link_paths {
                println!("cargo:rustc-link-search=native={}", path.display());
            }
        } else {
            vendor = true;
        }
    } else {
        vendor = true;
    }

    // Linking native Hoppet failed, therefore build it
    if vendor {
        let hoppet = cmake::Config::new("hoppet")
            .define("HOPPET_BUILD_EXAMPLES", "OFF")
            .define("HOPPET_ENABLE_TESTING", "OFF")
            .define("HOPPET_BUILD_BENCHMARK", "OFF")
            .env("CMAKE_Fortran_FLAGS", "-frecursive -fcheck=no-recursive -fPIC")
            .env("CMAKE_Fortran_FLAGS_RELEASE", "-frecursive -fcheck=no-recursive -fPIC")
            .profile("Release")
            .build();
        println!("cargo:rustc-link-search=native={}", hoppet.join("lib").display());
        println!("cargo:rustc-link-search=native={}", hoppet.join("lib64").display());
    }

    println!("cargo:rustc-link-lib=static=hoppet");
    println!("cargo:rustc-link-lib=dylib=gfortran");
    println!("cargo:rustc-link-lib=dylib=stdc++");
}
