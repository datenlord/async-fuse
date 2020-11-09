use memfs::MemFs;

use std::env;
use std::path::PathBuf;

use anyhow::Result;
use structopt::StructOpt;
use tracing::debug;

#[derive(Debug, StructOpt)]
struct Args {
    #[structopt(name = "TARGET", help = "The mount point of memfs")]
    target: PathBuf,
}

fn setup_tracing() {
    use tracing_error::ErrorSubscriber;
    use tracing_subscriber::{
        subscribe::CollectorExt,
        util::SubscriberInitExt,
        {fmt, EnvFilter},
    };

    tracing_subscriber::fmt()
        .event_format(fmt::format::Format::default().pretty())
        .with_env_filter(EnvFilter::from_default_env())
        .with_timer(fmt::time::ChronoLocal::rfc3339())
        .finish()
        .with(ErrorSubscriber::default())
        .init();
}

fn main() -> Result<()> {
    setup_tracing();
    let args = Args::from_args();
    async_std::task::block_on(run(args))?;
    Ok(())
}

#[allow(clippy::unit_arg)]
#[tracing::instrument(err)]
async fn run(args: Args) -> Result<()> {
    let cwd = env::current_dir()?;
    let target = cwd.join(&args.target);

    debug!(target = %target.display());

    let fs = MemFs::mount(target).initialize().await?;
    fs.run().await?;

    Ok(())
}
