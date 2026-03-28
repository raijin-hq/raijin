use std::fs;
use std::io::Write;
use std::path::Path;

fn main() {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let specs_dir = Path::new(&manifest_dir).join("specs");
    println!("cargo:rerun-if-changed=specs/");

    let out_dir = std::env::var("OUT_DIR").unwrap();
    let dest = Path::new(&out_dir).join("generated_specs.rs");
    let mut f = fs::File::create(&dest).unwrap();

    writeln!(
        f,
        "pub(crate) fn all_spec_strings() -> &'static [&'static str] {{"
    )
    .unwrap();
    writeln!(f, "    &[").unwrap();

    // Only embed top-level specs/ (Tier 1). External specs in specs/external/ are loaded at runtime.
    if specs_dir.exists() {
        let mut entries: Vec<_> = fs::read_dir(&specs_dir)
            .unwrap()
            .filter_map(|e| e.ok())
            .filter(|e| {
                e.path().extension().map_or(false, |ext| ext == "json")
                    && e.file_type().map_or(false, |ft| ft.is_file())
            })
            .collect();
        entries.sort_by_key(|e| e.file_name());

        for entry in entries {
            // Use absolute path so include_str! works from OUT_DIR
            let abs_path = entry.path().canonicalize().unwrap();
            writeln!(f, "        include_str!(\"{}\"),", abs_path.display()).unwrap();
        }
    }

    writeln!(f, "    ]").unwrap();
    writeln!(f, "}}").unwrap();
}
