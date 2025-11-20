use std::env;

fn main() {
    println!("cargo:rerun-if-changed=build.rs");

    // Try to use OpenBLAS first (more reliable with Cargo)
    if let Ok(openblas_libs) = pkg_config::probe_library("openblas") {
        println!("cargo:info=Using OpenBLAS for BLAS/LAPACK support");
        for path in openblas_libs.link_paths {
            println!("cargo:rustc-link-search=native={}", path.display());
        }
        for lib in openblas_libs.libs {
            println!("cargo:rustc-link-lib=dylib={lib}");
        }
        return;
    }

    // Fallback to Intel MKL if OpenBLAS is not available
    if let Ok(mkl_root) = env::var("MKLROOT") {
        println!("cargo:info=Using Intel MKL for BLAS/LAPACK support");

        // Standard MKL linkage for LP64 interface
        let mkl_lib_path = format!("{mkl_root}/lib/intel64");
        println!("cargo:rustc-link-search=native={mkl_lib_path}");

        // Link MKL libraries in correct order
        println!("cargo:rustc-link-lib=dylib=mkl_intel_lp64");
        println!("cargo:rustc-link-lib=dylib=mkl_sequential");
        println!("cargo:rustc-link-lib=dylib=mkl_core");
        println!("cargo:rustc-link-lib=dylib=iomp5"); // OpenMP runtime
        println!("cargo:rustc-link-lib=dylib=pthread");
        println!("cargo:rustc-link-lib=dylib=m");
        println!("cargo:rustc-link-lib=dylib=dl");

        return;
    }

    // Try system-wide MKL installation
    let mkl_paths = [
        "/opt/intel/oneapi/mkl/latest/lib/intel64",
        "/usr/lib/x86_64-linux-gnu",
        "/usr/local/lib",
    ];

    for mkl_path in &mkl_paths {
        if std::path::Path::new(mkl_path).exists() {
            println!("cargo:info=Found MKL at {mkl_path}");
            println!("cargo:rustc-link-search=native={mkl_path}");

            // Try different MKL linkage combinations
            let mkl_libs = [
                vec!["mkl_intel_lp64", "mkl_sequential", "mkl_core"],
                vec!["mkl_intel_lp64", "mkl_gnu_thread", "mkl_core"],
                vec!["mkl_intel_lp64", "mkl_intel_thread", "mkl_core"],
            ];

            for libs in mkl_libs {
                let mut success = true;
                for lib in &libs {
                    let lib_path = format!("{mkl_path}/lib{lib}.so");
                    if std::path::Path::new(&lib_path).exists() {
                        println!("cargo:rustc-link-lib=dylib={lib}");
                    } else {
                        success = false;
                        break;
                    }
                }

                if success {
                    println!("cargo:rustc-link-lib=dylib=iomp5");
                    println!("cargo:rustc-link-lib=dylib=pthread");
                    println!("cargo:rustc-link-lib=dylib=m");
                    println!("cargo:rustc-link-lib=dylib=dl");
                    return;
                }
            }
        }
    }

    // Final fallback: try to link with system BLAS
    println!("cargo:warning=No BLAS library found, attempting to link with system BLAS");
    println!("cargo:rustc-link-lib=dylib=blas");
    println!("cargo:rustc-link-lib=dylib=lapack");
}
