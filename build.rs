use vergen_gitcl::{Emitter, GitclBuilder};

// build.rs main func
fn main() -> anyhow::Result<()> {
    let gitcl = GitclBuilder::default().branch(true).sha(true).build()?;
    Emitter::default().add_instructions(&gitcl)?.emit()?;

    Ok(())
}
