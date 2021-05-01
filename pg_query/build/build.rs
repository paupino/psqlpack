use protobuf_codegen_pure::{Codegen, Customize};
use std::env;
use std::path::PathBuf;
use std::process::{Command, Stdio};

const VERSION: &'static str = "13-2.0.4";

fn main() {
    let code_dir = compile_pg_query();

    let out_dir = format!("{}/pg_query", env::var("OUT_DIR").unwrap());
    if std::path::Path::new(&out_dir).exists() {
        std::fs::remove_dir_all(&out_dir).unwrap();
    }
    std::fs::create_dir(&out_dir).unwrap();
    Codegen::new()
        .out_dir(out_dir)
        .input(code_dir.join("protobuf/pg_query.proto"))
        .include(code_dir.join("protobuf"))
        .customize(Customize {
            gen_mod_rs: Some(true),
            ..Default::default()
        })
        .run()
        .expect("protoc");
}

fn compile_pg_query() -> PathBuf {
    let out_dir = PathBuf::from(env::var_os("OUT_DIR").unwrap());

    let tarball = format!("{}.tar.gz", VERSION);
    let url = format!("https://github.com/lfittl/libpg_query/archive/{}", tarball);
    let build_dir = out_dir.join(format!("libpg_query-{}", VERSION));

    if !out_dir.join(&tarball).exists() {
        run_command(Command::new("curl").arg("-OL").arg(url).current_dir(&out_dir));
    }

    if !build_dir.exists() {
        run_command(
            Command::new("tar")
                .arg("xzf")
                .arg(out_dir.join(tarball))
                .current_dir(&out_dir),
        );
    }

    let mut command = Command::new("make");
    command.env_remove("PROFILE").arg("-C").arg(&build_dir);
    if env::var("PROFILE").unwrap() == "debug" {
        command.arg("DEBUG=1");
    }
    run_command(&mut command);

    println!("cargo:rustc-link-search=native={}", build_dir.display());
    println!("cargo:rustc-link-lib=static=pg_query");
    build_dir
}

fn run_command(command: &mut Command) {
    let status = command
        .stdin(Stdio::null())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()
        .unwrap();
    assert!(status.success());
}
