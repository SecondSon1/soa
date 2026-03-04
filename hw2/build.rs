use std::env;
use std::path::Path;
use std::process::Command;

fn main() {
  println!("cargo:rerun-if-changed=resources/openapi.yaml");
  println!("cargo:rerun-if-changed=scripts/generate-openapi.sh");

  if env::var("SKIP_OPENAPI_GENERATION").as_deref() == Ok("1") {
    return;
  }

  let generated_marker = Path::new("src/openapi_generated/src/models.rs");
  let spec = Path::new("resources/openapi.yaml");

  let needs_generation = !generated_marker.exists() || {
    let spec_modified = spec.metadata().and_then(|m| m.modified());
    let generated_modified = generated_marker.metadata().and_then(|m| m.modified());
    match (spec_modified, generated_modified) {
      (Ok(s), Ok(g)) => s > g,
      _ => true,
    }
  };

  if !needs_generation {
    return;
  }

  let status = Command::new("bash")
    .arg("scripts/generate-openapi.sh")
    .status()
    .expect("failed to run scripts/generate-openapi.sh");

  assert!(
    status.success(),
    "OpenAPI generation failed. Run scripts/generate-openapi.sh manually and check Docker access."
  );
}
