use clap::Parser;

/// Dependency management tool for Protocol Buffers files.
#[derive(Debug, Parser)]
#[clap(version = "0.0.1")]
pub struct CliArgs {
    #[clap(subcommand)]
    pub cmd: Command,
    #[clap(short, long, default_value = "protofetch.toml")]
    ///location of the protofetch configuration toml
    pub module_location: String,
    #[clap(short, long, default_value = "protofetch.lock")]
    ///location of the protofetch lock file
    pub lockfile_location: String,
    #[clap(short, long, default_value = ".protofetch_cache")]
    ///location of the protofetch cache directory
    pub cache_directory: String,
}

#[derive(Debug, Parser)]
pub enum Command {
    ///Fetches protodep dependencies defined in the toml configuration file
    Fetch {
        #[clap(short, long)]
        ///forces re-creation of lock file
        force_lock: bool,
        ///Name of the dependencies source files directory
        #[clap(short, long, default_value = "dependencies")]
        source_output_directory: String,
        ///Name of the proto files directory
        #[clap(short, long, default_value = "proto_src")]
        proto_output_directory: String,
    },
    ///Creates a lock file based on toml configuration file
    Lock,
    ///Creates an init protofetch setup in provided directory and name
    Init {
        #[clap(default_value = ".")]
        directory: String,
        #[clap(short, long)]
        name: Option<String>,
    },
    ///Migrates a protodep toml file to a protofetch format
    Migrate {
        #[clap(default_value = ".")]
        directory: String,
        #[clap(short, long)]
        name: Option<String>,
    },
}