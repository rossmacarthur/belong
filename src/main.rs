mod app;
mod config;
mod output;
mod prelude;
mod preprocess;
mod renderer;
mod theme;
mod util;

use std::env;
use std::process;

use anyhow::Context;
use casual::{confirm, prompt};

use structopt::clap::AppSettings;
use structopt::StructOpt;

#[derive(Debug, StructOpt)]
enum Command {
    /// Initialize a new project.
    Init,
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

/// Prompt the user for a title.
fn title() -> String {
    prompt("What title would you like to give the project?\n").get()
}

/// Retrieve a user name from Git.
fn git_config_user_name() -> Option<String> {
    let output = process::Command::new("git")
        .args(&["config", "--get", "user.name"])
        .output()
        .ok()?;
    if output.status.success() {
        Some(String::from_utf8_lossy(&output.stdout).trim().to_string())
    } else {
        None
    }
}

/// Prompt the user for an author.
fn author() -> String {
    let text = "Who is the author of this project?";
    match git_config_user_name() {
        Some(default) => prompt(format!("{} [default: {}]\n", text, &default))
            .default(default)
            .get(),
        None => prompt(format!("{}\n", text)).get(),
    }
}

fn run() -> anyhow::Result<()> {
    let Opt { command } = Opt::from_args();
    femme::with_level(femme::LevelFilter::Debug);
    let current_dir = env::current_dir().context("could not determine current directory")?;

    match command {
        Command::Init => {
            println!(
                "🎉 Initializing a new project ...\n\nPlease answer the following questions to \
                 get started:\n"
            );

            app::Builder::new(current_dir)
                .title(title())
                .author(author())
                .gitignore(confirm("Would you like a .gitignore file to be created?"))
                .init()
                .context("failed to initialize project")?;

            println!(
                "\nAll done! ✨ 🍰 ✨\n\nRun `belong build --open` to build and open the project."
            )
        }
        Command::Build { open } => {
            let project = app::Project::from_path(current_dir)
                .context("failed to load project")?
                .preprocess()
                .context("failed to preprocess project")?;
            let output = project.render().context("failed to render project")?;
            output
                .to_path()
                .context("failed to write rendered project")?;
            if open {
                open::that(output.config().output_dir().join("index.html"))
                    .context("failed to open web page in browser")?;
            }
        }
    }

    Ok(())
}

fn main() {
    if let Err(err) = run() {
        log::error!("{:?}", err);
        process::exit(2);
    }
}
