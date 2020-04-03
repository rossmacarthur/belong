use std::env;

use anyhow::Context;

fn main() -> anyhow::Result<()> {
    let should_open = match env::args().nth(1).as_ref().map(String::as_ref) {
        Some("--open") => true,
        None => false,
        Some(_) => anyhow::bail!("unrecognized command line argument"),
    };

    let current_dir = env::current_dir().context("could not determine current directory")?;
    let project = belong::Project::from_path(current_dir).context("failed to load project")?;
    project.render().context("failed to render project")?;

    if should_open {
        open::that(project.output_dir().join("index.html"))
            .context("failed to open web page in broswer")?;
    }

    Ok(())
}
