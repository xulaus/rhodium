#![feature(if_let_guard)]
#![feature(pattern)]
#![feature(async_closure)]

use clap::Parser;
use post::Post;
use ramhorns::Template;

mod development_server;
mod index;
mod post;
mod render;
mod utils;

#[derive(Parser)]
enum Args {
    Render {
        inp: std::path::PathBuf,
        template: std::path::PathBuf,
    },
    Serve {
        site_root: std::path::PathBuf,
    },
}

#[tokio::main]
async fn main() -> color_eyre::eyre::Result<()> {
    color_eyre::install()?;

    match Args::parse() {
        Args::Render {
            inp: md_file,
            template: path,
        } => {
            let template = Template::new(std::fs::read_to_string(path)?)?;
            let post = Post::from_file(&std::path::PathBuf::from("/"), &md_file);
            template.render_to_writer(&mut std::io::stdout(), &post)?;
            Ok(())
        }
        Args::Serve { site_root } => Ok(development_server::serve_forever(site_root).await?),
    }
}
