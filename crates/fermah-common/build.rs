use anyhow::Result;
use vergen::{BuildBuilder, Emitter, RustcBuilder};

fn main() -> Result<()> {
    let build = BuildBuilder::default().build_timestamp(true).build()?;
    let rustc = RustcBuilder::default()
        .host_triple(true)
        .commit_hash(true)
        .build()?;

    Emitter::default()
        .add_instructions(&build)?
        .add_instructions(&rustc)?
        .emit()
}
