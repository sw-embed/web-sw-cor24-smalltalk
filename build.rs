use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};

const SMALLTALK_DIR: &str = "../sw-cor24-smalltalk";

/// (out_name, st_filename, optional .bas driver)
///
/// Single-file demos (driver = `None`): the `.st` file has a `main`
/// block, so `tools/stc.awk` emits a complete program — driver
/// stub at lines 1..99, image at 100+, main bytecode in DATA. We
/// just concat with `vm.bas`.
///
/// Dual-file demos (driver = `Some(...)`): the `.st` file declares
/// classes/methods only (no `main`); `stc.awk` emits image only,
/// and the hand-assembled BASIC driver provides the top-level
/// loop (currently only `repl`, whose driver is a BASIC `INPUT`
/// REPL).
const DEMOS: &[(&str, &str, Option<&str>)] = &[
    ("repl", "d5_calc.st", Some("d5_calc.bas")),
    ("hello", "hello.st", None),
    ("add", "d1_add.st", None),
    ("counter", "d2_counter.st", None),
    ("boolean", "d3_boolean.st", None),
    ("max", "d4_max.st", None),
    ("factorial", "d6_fact.st", None),
    ("bounded", "d7_bounded.st", None),
];

fn main() {
    let out_dir = std::env::var("OUT_DIR").unwrap();
    let out_path = Path::new(&out_dir);

    // Bundle the BASIC interpreter p-code image.
    let basic_p24 = std::fs::read("assets/basic.p24").expect("assets/basic.p24");
    std::fs::write(out_path.join("basic.p24"), &basic_p24).unwrap();
    println!("cargo:rerun-if-changed=assets/basic.p24");

    let st = Path::new(SMALLTALK_DIR);
    let stc = st.join("tools/stc.awk");
    let vm_path = st.join("src/vm.bas");
    let vm = std::fs::read_to_string(&vm_path)
        .unwrap_or_else(|e| panic!("read {}: {e}", vm_path.display()));
    println!("cargo:rerun-if-changed={}", vm_path.display());
    println!("cargo:rerun-if-changed={}", stc.display());

    for (out_name, st_file, drv_file) in DEMOS {
        let st_path = st.join("examples").join(st_file);
        let st_src = std::fs::read_to_string(&st_path)
            .unwrap_or_else(|e| panic!("read {}: {e}", st_path.display()));

        let img = compile_st(&stc, &st_src, st_file);

        let mut bundle = String::new();
        bundle.push_str(img.trim_end());
        bundle.push('\n');
        bundle.push_str(vm.trim_end());
        bundle.push('\n');

        if let Some(drv) = drv_file {
            let drv_path = st.join("examples").join(drv);
            let drv_src = std::fs::read_to_string(&drv_path)
                .unwrap_or_else(|e| panic!("read {}: {e}", drv_path.display()));
            bundle.push_str(&strip_trailing_repl_cmds(&drv_src));
            println!("cargo:rerun-if-changed={}", drv_path.display());
        }

        if !bundle.ends_with('\n') {
            bundle.push('\n');
        }

        std::fs::write(out_path.join(format!("{out_name}.bas")), &bundle).unwrap();
        println!("cargo:rerun-if-changed={}", st_path.display());
    }

    // Build metadata for the footer.
    let sha = Command::new("git")
        .args(["rev-parse", "--short", "HEAD"])
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_else(|_| "unknown".into());

    let host = Command::new("hostname")
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_else(|_| "unknown".into());

    let timestamp = Command::new("date")
        .args(["-u", "+%Y-%m-%dT%H:%M:%SZ"])
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_else(|_| "unknown".into());

    println!("cargo:rustc-env=BUILD_SHA={sha}");
    println!("cargo:rustc-env=BUILD_HOST={host}");
    println!("cargo:rustc-env=BUILD_TIMESTAMP={timestamp}");
}

fn compile_st(stc: &Path, st_src: &str, label: &str) -> String {
    let mut child = Command::new("awk")
        .arg("-f")
        .arg(stc)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()
        .unwrap_or_else(|e| panic!("spawn awk for {label}: {e}"));
    child
        .stdin
        .as_mut()
        .unwrap()
        .write_all(st_src.as_bytes())
        .unwrap_or_else(|e| panic!("write stdin for {label}: {e}"));
    let output = child
        .wait_with_output()
        .unwrap_or_else(|e| panic!("wait for stc.awk on {label}: {e}"));
    if !output.status.success() {
        panic!("stc.awk failed for {label}: status {}", output.status);
    }
    String::from_utf8(output.stdout)
        .unwrap_or_else(|_| panic!("non-utf8 from stc.awk for {label}"))
}

fn strip_trailing_repl_cmds(src: &str) -> String {
    let mut lines: Vec<&str> = src.lines().collect();
    while let Some(last) = lines.last() {
        let t = last.trim();
        if t.is_empty() || t.eq_ignore_ascii_case("RUN") || t.eq_ignore_ascii_case("BYE") {
            lines.pop();
        } else {
            break;
        }
    }
    let mut out = lines.join("\n");
    out.push('\n');
    out
}
