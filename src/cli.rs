use clap::{Args, Parser, Subcommand};

/// Install websites as apps that feel native on GNOME.
#[derive(Debug, Parser)]
#[command(name = crate::paths::APP_NAME, version)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Install a website as an app
    Install(InstallArgs),
    /// List installed apps
    List(ListArgs),
    /// Uninstall an app
    Remove(RemoveArgs),
}

#[derive(Debug, Args)]
pub struct InstallArgs {
    /// Site to install, e.g. `github.com` or `https://github.com/notifications`
    pub url: String,

    /// App id (lowercase letters, digits and hyphens); derived from the host by default
    #[arg(long)]
    pub id: Option<String>,

    /// App name, instead of the one the site declares
    #[arg(long)]
    pub name: Option<String>,

    /// Icon to use instead of the site's own (local file or URL)
    #[arg(long, value_name = "PATH_OR_URL")]
    pub icon: Option<String>,

    /// Give the app its own browser profile instead of sharing the main one
    #[arg(long)]
    pub isolated: bool,

    /// Replace an app that is already installed (keeps its isolated profile)
    #[arg(long)]
    pub force: bool,

    /// Show what would be installed without writing anything
    #[arg(long)]
    pub dry_run: bool,
}

#[derive(Debug, Args)]
pub struct ListArgs {
    /// Print JSON instead of a table
    #[arg(long)]
    pub json: bool,
}

#[derive(Debug, Args)]
pub struct RemoveArgs {
    /// Id of the app to remove (see `winkle list`)
    pub id: String,

    /// Also delete the app's isolated profile (logins and site data)
    #[arg(long)]
    pub purge: bool,

    /// Confirm with a dialog and report with a notification (used by the app's Uninstall action)
    #[arg(long)]
    pub interactive: bool,
}
