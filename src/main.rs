mod cli;
mod html;
mod protocol;
mod server;

use anyhow::Result;

fn main() -> Result<()> {
    cli::run()
}
