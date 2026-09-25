use std::{env, path::Path, process::Command};
fn main() {
    let core = Path::new(env!("CARGO_MANIFEST_DIR")).join("../orchiddb");
    let git = |args: &[&str]| {
        Command::new("git")
            .arg("-C")
            .arg(&core)
            .args(args)
            .output()
            .ok()
            .filter(|o| o.status.success())
            .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_owned())
    };
    let expected = std::fs::read_to_string("CORE_REVISION").expect("CORE_REVISION");
    let head = git(&["rev-parse", "HEAD"]).unwrap_or_else(|| "unknown".into());
    let dirty = git(&["status", "--porcelain", "--untracked-files=normal"])
        .map(|s| !s.is_empty())
        .unwrap_or(true);
    if env::var("ORCHIDDB_RELEASE_BUILD").as_deref() == Ok("1") {
        assert_eq!(
            head,
            expected.trim(),
            "release requires pinned core revision"
        );
        assert!(!dirty, "release requires a clean core checkout");
    }
    let revision = if dirty { format!("{head}-dirty") } else { head };
    println!("cargo:rustc-env=ORCHIDDB_BUILT_CORE_REVISION={revision}");
    println!("cargo:rerun-if-env-changed=ORCHIDDB_RELEASE_BUILD");
    println!("cargo:rerun-if-changed=CORE_REVISION");
    // Observe core implementation/HEAD changes in sibling development checkouts.
    println!("cargo:rerun-if-changed={}", core.join("src").display());
    println!(
        "cargo:rerun-if-changed={}",
        core.join(".git/HEAD").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        core.join(".git/refs/heads").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        core.join(".git/index").display()
    );
}
