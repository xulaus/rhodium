#![feature(if_let_guard)]
#![feature(let_chains)]
#![feature(pattern)]
#![feature(async_closure)]

use clap::Parser;
use index::Index;
use post::Post;
use ramhorns::Template;
use syntect::parsing::SyntaxSet;
use std::path::{PathBuf, Path};

mod development_server;
mod index;
mod post;
mod render;
mod utils;

#[derive(Parser)]
enum Args {
    Build {
        #[arg(long)]
        site_root: Option<PathBuf>,
        #[arg(long, default_value="_site")]
        build_dir: PathBuf,
    },
    Serve {
        #[arg(long)]
        site_root: Option<PathBuf>,
    },
}

fn load_syntax_set(site_root: &Path) -> color_eyre::eyre::Result<SyntaxSet> {
    println!("Loading Syntax Set....");
    use syntect::parsing::SyntaxDefinition;
    let mut ssb = SyntaxSet::load_defaults_newlines().into_builder();
    let syntax_folder = site_root.join("_config/syntaxes");

    if let Ok(folder) = std::fs::read_dir(syntax_folder) {
        for file in folder {
            if let Ok(file) = file && file.path().extension().and_then(std::ffi::OsStr::to_str) == Some("sublime-syntax") {
                let path = file.path();
                println!("Loading {path:?}...");
                let file_content = std::fs::read_to_string(path)?;
                let def = SyntaxDefinition::load_from_str(&file_content, true, None)?;
                ssb.add(def);
            }
        }
    }
    let ps = ssb.build();
    println!("Done");
    Ok(ps)
}

#[tokio::main]
async fn main() -> color_eyre::eyre::Result<()> {
    color_eyre::install()?;

    match Args::parse() {
        Args::Build {
            site_root,
            build_dir,
        } => {
            let site_root = site_root.unwrap_or(".".into());
            let syntax_set: SyntaxSet = load_syntax_set(&site_root)?;
            let post_template = {
                let template_path = site_root.join("_config/layouts/post.hbs");
                let template_source =
                    std::fs::read_to_string(&template_path).unwrap_or_else(|_| {
                        panic!("Couldn't find index template at {:?}", &template_path)
                    });
                Template::new(template_source)?
            };
            let index_template = {
                let template_path = site_root.join("_config/layouts/index.hbs");
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

                let post = Post::from_file(&site_root, md_file, &syntax_set)?;
                let mut out_file = std::io::BufWriter::new(std::fs::File::create(out_file_path)?);
                post_template.render_to_writer(&mut out_file, &post)?;
            }

            let mut out_file = {
                let index_path = build_dir.join("index.html");
                std::io::BufWriter::new(std::fs::File::create(index_path)?)
            };
            index_template
                .render_to_writer(&mut out_file, &Index::from_file_list(&site_root, &all_site, &syntax_set))?;

            Ok(())
        }
        Args::Serve { site_root } => {
            let site_root = site_root.unwrap_or(".".into());
            let syntax_set: SyntaxSet = load_syntax_set(&site_root)?;
            Ok(development_server::serve_forever(site_root, syntax_set).await?)
        }
    }
}
