// `sqlx::migrate!()` compiles the migrations into the binary: build again when one changes.
fn main() {
    println!("cargo:rerun-if-changed=migrations");
}
