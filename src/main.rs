use std::{
    error::Error,
    path::{Path, PathBuf},
};

use clap::Parser;

use protofetch::{
    cache::ProtofetchGitCache,
    cli,
    cli::{args::CliArgs, command_handlers, HttpGitAuth},
};
use tracing_subscriber::filter::LevelFilter;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

fn main() {
    // -- NOTE --------------------------------------------------------------------------------------
    // The filtering methods on a stack of Layers are evaluated in a top-down order, starting
    // with the outermost Layer and ending with the wrapped Subscriber. If any layer returns
    // false from its enabled method, or Interest::never() from its register_callsite method,
    // filter evaluation will short-circuit and the span or event will be disabled.
    //
    // https://docs.rs/tracing-subscriber/latest/tracing_subscriber/layer/index.html#global-filtering
    // ----------------------------------------------------------------------------------------------
    let filter = tracing_subscriber::EnvFilter::builder()
        .with_env_var("RUST_LOG")
        .with_default_directive(LevelFilter::INFO.into())
        .from_env_lossy();

    // Initialize stdout output;
    let stdout_layer = tracing_subscriber::fmt::layer()
        .with_line_number(true)
        .with_file(true)
        .with_thread_ids(false)
        .with_target(true)
        .with_ansi(true);

    tracing_subscriber::registry()
        .with(stdout_layer)
        .with(filter)
        .try_init()
        .unwrap();

    if let Err(e) = run() {
        tracing::error!("{}", e)
    }
}

fn run() -> Result<(), Box<dyn Error>> {
    let cli_args: CliArgs = CliArgs::parse();
    let home_dir =
        home::home_dir().expect("Could not find home dir. Please define $HOME env variable.");
    let cache_path = home_dir.join(PathBuf::from(&cli_args.cache_directory));
    let git_auth = HttpGitAuth::resolve_git_auth(cli_args.username, cli_args.password)?;
    let cache = ProtofetchGitCache::new(cache_path, git_auth)?;
    let module_path = Path::new(&cli_args.module_location);
    let lockfile_path = Path::new(&cli_args.lockfile_location);
    let proto_output_directory = Path::new(&cli_args.output_proto_directory);
    match cli_args.cmd {
        cli::args::Command::Fetch {
            force_lock,
            repo_output_directory: source_output_directory,
        } => {
            let dependencies_out_dir = Path::new(&source_output_directory);
            let proto_output_directory = Path::new(&proto_output_directory);

            command_handlers::do_fetch(
                force_lock,
                &cache,
                module_path,
                lockfile_path,
                dependencies_out_dir,
                proto_output_directory,
            )
        }
        cli::args::Command::Lock => {
            command_handlers::do_lock(&cache, module_path, lockfile_path)?;
            Ok(())
        }
        cli::args::Command::Init { directory, name } => {
            command_handlers::do_init(&directory, name.as_deref(), &cli_args.module_location)
        }
        cli::args::Command::Migrate { directory, name } => {
            command_handlers::do_migrate(&directory, name.as_deref(), &cli_args.module_location)
        }
        cli::args::Command::Clean => {
            command_handlers::do_clean(lockfile_path, proto_output_directory)
        }
        cli::args::Command::ClearCache => command_handlers::do_clear_cache(&cache),
    }
}
