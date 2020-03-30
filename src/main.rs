use std::env;

use anyhow::Context;

fn main() -> anyhow::Result<()> {
    let current_dir = env::current_dir().context("could not determine current directory")?;
    belong::Project::from_path(current_dir)
        .context("failed to load project")?
        .render()
        .context("failed to render project")?;
    Ok(())
}
