use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use colored::Colorize;
use dialoguer::{Confirm, Input};
use std::env;
use std::path::PathBuf;

use git_anon::{AnonymousIdentity, GitAnon, config::Config};

#[derive(Parser)]
#[command(
    name = "git-anon",
    version,
    author = "Ludwig",
    about = "Anonymize git repositories for public sharing",
    long_about = "A tool for anonymizing git repositories by rewriting commit history with anonymous identities"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    #[arg(short, long, help = "Path to git repository")]
    repo: Option<PathBuf>,

    #[arg(short, long, help = "Skip confirmation prompts")]
    yes: bool,

    #[arg(short, long, help = "Verbose output")]
    verbose: bool,

    #[arg(short, long, help = "Show what would be done without making changes")]
    dry_run: bool,
}

#[derive(Subcommand)]
enum Commands {
    #[command(about = "Squash all commits into a single anonymous commit")]
    Squash {
        #[arg(short, long, help = "Commit message for the squashed commit")]
        message: Option<String>,
    },

    #[command(about = "Push to remote with anonymized commits")]
    Push {
        #[arg(help = "Remote name to push to")]
        remote: String,

        #[arg(help = "Branch to push")]
        branch: Option<String>,

        #[arg(short, long, help = "Force push")]
        force: bool,
    },

    #[command(about = "Fully clean and anonymize repository")]
    Clean,

    #[command(about = "Manage configuration")]
    Config {
        #[command(subcommand)]
        action: ConfigAction,
    },
}

#[derive(Subcommand)]
enum ConfigAction {
    #[command(about = "Show current configuration")]
    Show,

    #[command(about = "Set anonymous identity")]
    SetIdentity {
        #[arg(help = "Name for anonymous identity")]
        name: Option<String>,

        #[arg(help = "Email for anonymous identity")]
        email: Option<String>,
    },

    #[command(about = "Add or update remote configuration")]
    AddRemote {
        #[arg(help = "Remote alias (e.g., 'radicle')")]
        alias: String,

        #[arg(help = "Git remote name (e.g., 'rad')")]
        remote_name: String,

        #[arg(help = "Identity to use for this remote")]
        identity: Option<String>,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    let repo_path = cli
        .repo
        .unwrap_or_else(|| env::current_dir().expect("Failed to get current directory"));

    match cli.command {
        Commands::Config { action } => handle_config(action),
        _ => {
            let config = Config::load().context("Failed to load configuration")?;
            let identity = get_identity_for_command(&config, &cli.command)?;
            let git_anon = GitAnon::new(repo_path, identity)?;

            match cli.command {
                Commands::Squash { message } => git_anon.squash(message, cli.yes, cli.dry_run),
                Commands::Push {
                    remote,
                    branch,
                    force,
                } => git_anon.push(&remote, branch, force, cli.dry_run),
                Commands::Clean => git_anon.clean(cli.yes, cli.dry_run),
                Commands::Config { .. } => unreachable!(),
            }
        }
    }
}

fn get_identity_for_command(config: &Config, command: &Commands) -> Result<AnonymousIdentity> {
    match command {
        Commands::Push { remote, .. } => Ok(config.get_remote_identity(remote)),
        _ => Ok(AnonymousIdentity {
            name: config.anonymous_identity.name.clone(),
            email: config.anonymous_identity.email.clone(),
        }),
    }
}

fn handle_config(action: ConfigAction) -> Result<()> {
    match action {
        ConfigAction::Show => {
            let config = Config::load()?;
            let config_path = Config::config_path()?;

            println!(
                "Configuration file: {}",
                config_path.display().to_string().cyan()
            );
            println!();
            println!("Anonymous identity:");
            println!("  Name:  {}", config.anonymous_identity.name.green());
            println!("  Email: {}", config.anonymous_identity.email.green());
            println!();

            if !config.remotes.is_empty() {
                println!("Remotes:");
                for (alias, remote_config) in &config.remotes {
                    println!(
                        "  {} -> {} (identity: {})",
                        alias.yellow(),
                        remote_config.name.blue(),
                        remote_config.identity.green()
                    );
                }
            }
        }

        ConfigAction::SetIdentity { name, email } => {
            let mut config = Config::load()?;

            let is_interactive = name.is_none() || email.is_none();

            let name = name.unwrap_or_else(|| {
                Input::new()
                    .with_prompt("Anonymous name")
                    .default(config.anonymous_identity.name.clone())
                    .interact_text()
                    .unwrap_or_else(|_| config.anonymous_identity.name.clone())
            });

            let email = email.unwrap_or_else(|| {
                Input::new()
                    .with_prompt("Anonymous email")
                    .default(config.anonymous_identity.email.clone())
                    .interact_text()
                    .unwrap_or_else(|_| config.anonymous_identity.email.clone())
            });

            validate_identity(&name, &email)?;

            show_identity_changes(&config.anonymous_identity, &name, &email);

            if is_interactive && !confirm_changes("Update anonymous identity?")? {
                println!("Cancelled.");
                return Ok(());
            }

            config.anonymous_identity.name = name;
            config.anonymous_identity.email = email;
            config.save()?;

            println!("{} Updated anonymous identity", "✓".green());
        }

        ConfigAction::AddRemote {
            alias,
            remote_name,
            identity,
        } => {
            let mut config = Config::load()?;
            config.remotes.insert(
                alias.clone(),
                git_anon::config::RemoteConfig {
                    name: remote_name,
                    identity: identity.unwrap_or_else(|| "anonymous_identity".to_string()),
                },
            );
            config.save()?;

            println!(
                "{} Added remote configuration: {}",
                "✓".green(),
                alias.yellow()
            );
        }
    }

    Ok(())
}

fn validate_identity(name: &str, email: &str) -> Result<()> {
    if name.trim().is_empty() {
        anyhow::bail!("Name cannot be empty");
    }
    if email.trim().is_empty() {
        anyhow::bail!("Email cannot be empty");
    }
    if !email.contains('@') {
        anyhow::bail!("Email must contain @ symbol");
    }
    Ok(())
}

fn show_identity_changes(current: &git_anon::config::Identity, new_name: &str, new_email: &str) {
    println!("Current anonymous identity:");
    println!("  Name:  {}", current.name.yellow());
    println!("  Email: {}", current.email.yellow());
    println!();
    println!("New anonymous identity:");
    println!("  Name:  {}", new_name.green());
    println!("  Email: {}", new_email.green());
    println!();
}

fn confirm_changes(prompt: &str) -> Result<bool> {
    Ok(Confirm::new()
        .with_prompt(prompt)
        .default(true)
        .interact()?)
}
