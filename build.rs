use std::path::Path;
use std::process::Command;

const SMALLTALK_DIR: &str = "../sw-cor24-smalltalk";

// (demo_name, image_filename, driver_filename)
const DEMOS: &[(&str, &str, &str)] = &[
    ("d1_add", "image_d1.bas", "d1_add.bas"),
    ("d2_counter", "image_d2.bas", "d2_counter.bas"),
    ("d3_boolean", "image_d3.bas", "d3_boolean.bas"),
    ("d4_max", "image_d4.bas", "d4_max.bas"),
    ("d5_calc", "image_d5.bas", "d5_calc.bas"),
    ("d6_fact", "image_d6.bas", "d6_fact.bas"),
    ("d7_bounded", "image_d7.bas", "d7_bounded.bas"),
];

fn main() {
    let out_dir = std::env::var("OUT_DIR").unwrap();
    let out_path = Path::new(&out_dir);

    // Bundle the BASIC interpreter p-code image.
    let basic_p24 = std::fs::read("assets/basic.p24").expect("assets/basic.p24");
    std::fs::write(out_path.join("basic.p24"), &basic_p24).unwrap();
    println!("cargo:rerun-if-changed=assets/basic.p24");

    // Build per-demo .bas bundles by concatenating image + vm + driver.
    // Trailing `RUN` / `BYE` REPL commands are stripped: the runtime
    // (src/runner.rs) appends the right ones based on interactive mode.
    let st = Path::new(SMALLTALK_DIR);
    let vm_path = st.join("src/vm.bas");
    let vm = std::fs::read_to_string(&vm_path)
        .unwrap_or_else(|e| panic!("read {}: {e}", vm_path.display()));
    println!("cargo:rerun-if-changed={}", vm_path.display());

    for (name, image, driver) in DEMOS {
        let img_path = st.join("src").join(image);
        let drv_path = st.join("examples").join(driver);
        let img = std::fs::read_to_string(&img_path)
            .unwrap_or_else(|e| panic!("read {}: {e}", img_path.display()));
        let drv = std::fs::read_to_string(&drv_path)
            .unwrap_or_else(|e| panic!("read {}: {e}", drv_path.display()));

        let mut bundle = String::with_capacity(img.len() + vm.len() + drv.len() + 4);
        bundle.push_str(img.trim_end());
        bundle.push('\n');
        bundle.push_str(vm.trim_end());
        bundle.push('\n');
        bundle.push_str(&strip_trailing_repl_cmds(&drv));
        if !bundle.ends_with('\n') {
            bundle.push('\n');
        }

        std::fs::write(out_path.join(format!("{name}.bas")), &bundle).unwrap();
        println!("cargo:rerun-if-changed={}", img_path.display());
        println!("cargo:rerun-if-changed={}", drv_path.display());
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
