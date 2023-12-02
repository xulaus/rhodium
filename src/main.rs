#![feature(if_let_guard)]
#![feature(pattern)]
#![feature(async_closure)]

use clap::Parser;
use index::Index;
use post::Post;
use ramhorns::Template;

mod development_server;
mod index;
mod post;
mod render;
mod utils;

#[derive(Parser)]
enum Args {
    Build {
        site_root: std::path::PathBuf,
        #[arg(long)]
        build_dir: std::path::PathBuf,
    },
    Serve {
        site_root: std::path::PathBuf,
    },
}

#[tokio::main]
async fn main() -> color_eyre::eyre::Result<()> {
    color_eyre::install()?;

    match Args::parse() {
        Args::Build {
            site_root,
            build_dir,
        } => {
            let post_template = {
                let template_path = site_root.join("_layouts/post.hbs");
                let template_source =
                    std::fs::read_to_string(&template_path).unwrap_or_else(|_| {
                        panic!("Couldn't find index template at {:?}", &template_path)
                    });
                Template::new(template_source)?
            };
            let index_template = {
                let template_path = site_root.join("_layouts/index.hbs");
                let template_source =
                    std::fs::read_to_string(&template_path).unwrap_or_else(|_| {
                        panic!("Couldn't find post template at {:?}", &template_path)
                    });
                Template::new(template_source)?
            };

            let all_site = utils::files_within(&site_root)?;
            for md_file in &all_site {
                let mut out_file_path = build_dir.join(md_file);
                out_file_path.set_extension("html");

                std::fs::create_dir_all(out_file_path.parent().unwrap())?;

                let post = Post::from_file(&site_root, md_file)?;
                let mut out_file = std::io::BufWriter::new(std::fs::File::create(out_file_path)?);
                post_template.render_to_writer(&mut out_file, &post)?;
            }

            let mut out_file = {
                let index_path = build_dir.join("index.html");
                std::io::BufWriter::new(std::fs::File::create(index_path)?)
            };
            index_template
                .render_to_writer(&mut out_file, &Index::from_file_list(&site_root, &all_site))?;

            Ok(())
        }
        Args::Serve { site_root } => Ok(development_server::serve_forever(site_root).await?),
    }
}
