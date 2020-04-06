use std::env;

use anyhow::Context;

use structopt::{clap::AppSettings, StructOpt};

#[derive(Debug, StructOpt)]
enum Command {
    /// Initialize a new project.
    New,
    /// Build the project from Markdown files.
    Build {
        /// Opens the compiled project in the default web browser.
        #[structopt(long)]
        open: bool,
    },
}

#[derive(Debug, StructOpt)]
#[structopt(
    global_settings = &[
        AppSettings::DeriveDisplayOrder,
        AppSettings::DisableHelpSubcommand,
        AppSettings::VersionlessSubcommands,
    ],
)]
struct Opt {
    #[structopt(subcommand)]
    command: Command,
}

fn main() -> anyhow::Result<()> {
    let Opt { command } = Opt::from_args();
    let current_dir = env::current_dir().context("could not determine current directory")?;

    match command {
        Command::New => {
            belong::Builder::new(current_dir)
                .build()
                .context("failed to initialize project")?;
        }
        Command::Build { open } => {
            let project =
                belong::Project::from_path(current_dir).context("failed to load project")?;
            project.render().context("failed to render project")?;
            if open {
                open::that(project.output_dir().join("index.html"))
                    .context("failed to open web page in browser")?;
            }
        }
    }

    Ok(())
}
