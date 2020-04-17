mod types;

use std::collections::HashMap;
use std::fs::File;
use std::env;
use std::io::{BufReader, BufWriter, Write};
use std::process::{Command, Stdio};
use std::path::PathBuf;

use types::{Enum};

const VERSION: &'static str = "10-1.0.2";

fn main() {
    let data = compile_pg_query();
    generate_pg_query_types(&data);
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
        run_command(Command::new("tar").arg("xzf").arg(out_dir.join(tarball)).current_dir(&out_dir));
    }

    let mut command = Command::new("make");
    command.env_remove("PROFILE").arg("-C").arg(&build_dir);
    if env::var("PROFILE").unwrap() == "debug" {
        command.arg("DEBUG=1");
    }
    run_command(&mut command);

    println!("cargo:rustc-link-search=native={}", build_dir.display());
    println!("cargo:rustc-link-lib=static=pg_query");
    build_dir.join("srcdata")
}

fn generate_pg_query_types(dir: &PathBuf) {
    //let struct_defs = File::open(dir.join("struct_defs.json")).unwrap();
    //let struct_defs = BufReader::new(struct_defs);

    //let struct_defs: HashMap<String, HashMap<String, Struct>> =
    //    serde_json::from_reader(struct_defs).unwrap();

    let enum_defs = File::open(dir.join("enum_defs.json")).unwrap();
    let enum_defs = BufReader::new(enum_defs);
    let enum_defs: HashMap<String, HashMap<String, Enum>> =
        serde_json::from_reader(enum_defs).unwrap();
    let out_dir = PathBuf::from(env::var_os("OUT_DIR").unwrap());
    let out = File::create(out_dir.join("enums.rs")).unwrap();
    let mut out = BufWriter::new(out);
    make_enums(&enum_defs, &mut out);
}

fn make_enums(enum_defs: &HashMap<String, HashMap<String, Enum>>, out: &mut BufWriter<File>) {
    for (name, def) in &enum_defs["nodes/parsenodes"] {
        if let Some(comment) = &def.comment {
            write!(out, "{}", comment).unwrap();
        }
        write!(out, "pub enum {} {{\n", name).unwrap();

        for value in &def.values {
            if let Some(comment) = &value.comment {
                write!(out, "    {}\n", comment).unwrap();
            }
            if let Some(name) = &value.name {
                write!(out, "    {},\n", name).unwrap();
            }
        }
        write!(out, "}}\n\n").unwrap();
    }
}

fn run_command(command: &mut Command) {
    let status = command.stdin(Stdio::null())
                        .stdout(Stdio::inherit())
                        .stderr(Stdio::inherit())
                        .status()
                        .unwrap();
    assert!(status.success());
}