//! macrandom CLI entry point.

use clap::{CommandFactory, Parser};
use clap_complete::generate;
use macrandom::cli::{Cli, Commands};
use macrandom::commands::RandomizeArgs;
use macrandom::commands::{cmd_list, cmd_randomize, cmd_restore, cmd_status, cmd_vendor};
use macrandom::config::{Config, Paths};
use macrandom::error::MacrandomError;
use macrandom::output::{stderr_error, OutputOpts};
use std::io;
use std::process::ExitCode;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tracing_subscriber::EnvFilter;

fn main() -> ExitCode {
    let cli = Cli::parse();

    init_logging(cli.verbose, cli.quiet, cli.json);

    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();
    let _ = ctrlc::set_handler(move || {
        r.store(false, Ordering::SeqCst);
        eprintln!("\ninterrupted");
        std::process::exit(130);
    });

    match run(cli) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            stderr_error(&e);
            ExitCode::from(e.exit_code() as u8)
        }
    }
}

fn init_logging(verbose: u8, quiet: bool, json: bool) {
    let level = if quiet {
        "error"
    } else {
        match verbose {
            0 => "warn",
            1 => "info",
            2 => "debug",
            _ => "trace",
        }
    };
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new(format!("macrandom={level},info")));

    let builder = tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_writer(io::stderr)
        .with_target(false);

    if json {
        let _ = builder.json().try_init();
    } else {
        let _ = builder.try_init();
    }
}

fn run(cli: Cli) -> Result<(), MacrandomError> {
    let paths = if let Some(ref cfg_path) = cli.config {
        let mut p = Paths::xdg()?;
        p.config_file = cfg_path.clone();
        p
    } else {
        Paths::xdg()?
    };

    let cfg = Config::load(&paths.config_file)?;
    let opts = OutputOpts {
        json: cli.json,
        quiet: cli.quiet,
    };

    match cli.command {
        Commands::List { all } => cmd_list(&opts, all, &cfg.extra_skip),
        Commands::Status => cmd_status(&opts, &paths),
        Commands::Randomize {
            iface,
            all,
            mode,
            force,
            persistent,
            no_persistent,
        } => {
            let use_persistent = if no_persistent {
                false
            } else {
                persistent || cfg.persistent_nm
            };
            cmd_randomize(
                &opts,
                &paths,
                &cfg,
                RandomizeArgs {
                    iface: iface.as_deref(),
                    all,
                    mode: mode.into(),
                    force,
                    persistent: use_persistent,
                    dry_run: cli.dry_run,
                },
            )
        }
        Commands::Restore {
            iface,
            force,
            no_clear_nm,
        } => cmd_restore(
            &opts,
            &paths,
            &cfg,
            &iface,
            force,
            !no_clear_nm,
            cli.dry_run,
        ),
        Commands::Vendor {
            iface,
            force,
            persistent,
        } => cmd_vendor(
            &opts,
            &paths,
            &cfg,
            &iface,
            force,
            persistent || cfg.persistent_nm,
            cli.dry_run,
        ),
        Commands::Completions { shell } => {
            let mut cmd = Cli::command();
            let name = cmd.get_name().to_string();
            generate(
                clap_complete::Shell::from(shell),
                &mut cmd,
                name,
                &mut io::stdout(),
            );
            Ok(())
        }
    }
}
