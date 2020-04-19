mod types;

use std::collections::HashMap;
use std::fs::File;
use std::env;
use std::io::{BufReader, BufWriter, Write};
use std::process::{Command, Stdio};
use std::path::PathBuf;

use types::{Enum, Struct};

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

    // Common out dir
    let out_dir = PathBuf::from(env::var_os("OUT_DIR").unwrap());
    let out_file = File::create(out_dir.join("types.rs")).unwrap();
    let mut out_file = BufWriter::new(out_file);
    write_header(&mut out_file);

    // First do enums
    let enum_defs = File::open(dir.join("enum_defs.json")).unwrap();
    let enum_defs = BufReader::new(enum_defs);
    let enum_defs: HashMap<String, HashMap<String, Enum>> =
        serde_json::from_reader(enum_defs).unwrap();
    make_enums(&enum_defs, &mut out_file);

    // Next do structs
    let struct_defs = File::open(dir.join("struct_defs.json")).unwrap();
    let struct_defs = BufReader::new(struct_defs);
    let struct_defs: HashMap<String, HashMap<String, Struct>> =
        serde_json::from_reader(struct_defs).unwrap();
    make_nodes(&struct_defs, &mut out_file);

    write_footer(&mut out_file);
}

fn write_header(out: &mut BufWriter<File>) {
    write!(out, "pub use __pg_query::*;\n\n").unwrap();
    write!(out, "mod __pg_query {{\n").unwrap();
    write!(out, "    #![allow(non_snake_case, non_camel_case_types, unused_mut, unused_variables, unused_imports, unused_parens)]\n").unwrap();
}

fn write_footer(out: &mut BufWriter<File>) {
    write!(out, "}}\n").unwrap();
}

fn make_enums(enum_defs: &HashMap<String, HashMap<String, Enum>>, out: &mut BufWriter<File>) {
    for (name, def) in &enum_defs["nodes/parsenodes"] {
        write!(out, "    pub enum {} {{\n", name).unwrap();

        for value in &def.values {
            if let Some(comment) = &value.comment {
                write!(out, "        {}\n", comment).unwrap();
            }
            if let Some(name) = &value.name {
                write!(out, "        {},\n", name).unwrap();
            }
        }
        write!(out, "    }}\n\n").unwrap();
    }
}

fn make_nodes(struct_defs: &HashMap<String, HashMap<String, Struct>>, out: &mut BufWriter<File>) {
    write!(out, "    pub enum Node {{\n").unwrap();
    for (name, def) in &struct_defs["nodes/parsenodes"] {
        write!(out, "        {} {{\n", name).unwrap();

        for field in &def.fields {
            let (name, c_type) = match (&field.name, &field.c_type) {
                (&Some(ref name), &Some(ref c_type)) => (name, c_type),
                _ => continue,
            };

            if name == "type" {
                continue;
            }
            write!(out, "            {}: {},\n",
                   remove_reserved_keyword(name), c_to_rust_type(c_type)).unwrap();
        }

        write!(out, "        }},\n").unwrap();
    }

    write!(out, "    }}\n").unwrap();
}

fn remove_reserved_keyword(variable: &str) -> String {
    if is_reserved(variable) {
        format!("{}_", variable)
    } else {
        variable.into()
    }
}

fn is_reserved(variable: &str) -> bool {
    match variable {
        "abstract" |
        "become" |
        "box" |
        "do" |
        "final" |
        "macro" |
        "override" |
        "priv" |
        "try" |
        "typeof" |
        "unsized" |
        "virtual" |
        "yield" => true,
        _ => false,
    }
}

fn c_to_rust_type(c_type: &str) -> &str {
    match c_type {
        "uint32" => "u32",
        "bool" => "bool",
        "Node*" => "Box<Node>",
        "int" => "i32",
        "List*" => "Vec<Node>",
        "CreateStmt" => "Box<Node>",
        "int32" => "i32",
        "char*" => "String",
        "int16" => "i16",
        "char" => "u8",
        "SelectStmt*" => "Box<Node>",
        other => {
            if other.ends_with("*") {
                other[0..other.len() - 1].into()
            } else {
                other
            }
        }
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