use std::env;
use std::path::PathBuf;

fn main() {
    tauri_build::build();

    // 获取项目根目录 (src-tauri 的上一级)
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    let root_dir = PathBuf::from(&manifest_dir)
        .parent()
        .unwrap()
        .to_path_buf();

    // uv 虚拟环境位于 python_engine/.venv
    let venv_base = root_dir.join("python_engine").join(".venv");
    let python_exe = if cfg!(windows) {
        venv_base.join("Scripts").join("python.exe")
    } else {
        venv_base.join("bin").join("python")
    };

    // 仅在 venv 已存在时才设置 PYO3_PYTHON
    // 开发流程: 先 uv sync 创建环境 -> cargo build -> 运行
    if python_exe.exists() {
        println!("cargo:rustc-env=PYO3_PYTHON={}", python_exe.display());
        println!("cargo:warning=检测到 uv 虚拟环境: {:?}", python_exe);
    } else {
        // venv 不存在时只给警告，不阻塞构建
        // 但注意: 不用 abi3 时，pyo3-build-config 仍会尝试找 Python
        println!(
            "cargo:warning=未找到 uv 虚拟环境 {:?}，请先运行 'uv sync' 创建环境后再构建",
            python_exe
        );
    }

    // 自动将 numpy.libs 和 scipy.libs 目录添加到 rustc-link-search，
    // 使链接器能正确链接 Python 扩展模块依赖的 DLL
    if python_exe.exists() {
        let site_packages = venv_base
            .join("Lib")
            .join("site-packages");

        // 收集所有存在的 .libs 目录
        let libs_dirs = [
            "numpy.libs",
            "scipy.libs",
            "sklearn.libs",
            "llvmlite.libs",
        ];

        for libs_name in libs_dirs {
            let libs_path = site_packages.join(libs_name);
            if libs_path.exists() {
                println!(
                    "cargo:rustc-link-search=native={}",
                    libs_path.display()
                );
                println!(
                    "cargo:warning=添加 {} 链接搜索路径",
                    libs_name
                );
            }
        }
    }

    println!("cargo:rerun-if-changed=build.rs");
    println!(
        "cargo:rerun-if-changed={}",
        root_dir.join("python_engine").join("pyproject.toml").display()
    );
}