#![feature(if_let_guard)]
#![feature(pattern)]

use std::path::PathBuf;

use markdown::{mdast, to_mdast, ParseOptions};
use ramhorns::{Content, Template};
mod render;
mod utils;
use render::{mdast_into_str_builder, MarkdownError, Toc};

use clap::Parser;
#[derive(Parser)]
enum Args {
    Render {
        inp: std::path::PathBuf,
        template: std::path::PathBuf,
    },
    Serve {},
}

#[derive(Content, Debug)]
struct Post<'a> {
    title: &'a str,
    toc: Option<&'a str>,
    content: &'a str,
}

fn render_md_to_writer<W>(
    md_file: PathBuf,
    template: &Template,
    writer: &mut W,
) -> color_eyre::Result<()>
where
    W: std::io::Write,
{
    let md_string = std::fs::read_to_string(md_file)?;
    let md_ast = to_mdast(&md_string, &ParseOptions::gfm());

    let (tree, root) = match &md_ast {
        Ok(tree) if let mdast::Node::Root(root) = tree => (tree, root),
        Ok(_) => return Err(MarkdownError::InvalidRoot.into()),
        Err(err) => return Err(MarkdownError::ErrorParsing { wrapped: err.clone() }.into()),
    };
    let toc = Toc::from_mdast(root)?;
    let content = {
        let mut builder = vec![];
        mdast_into_str_builder(tree, &mut builder)?;
        builder.concat()
    };

    let toc_html = toc.to_html();
    let post = Post {
        title: &toc.name,
        content: &content,
        toc: toc_html.as_deref(),
    };
    Ok(template.render_to_writer(writer, &post)?)
}

fn main() -> color_eyre::eyre::Result<()> {
    color_eyre::install()?;

    match Args::parse() {
        Args::Render {
            inp: md_file,
            template: path,
        } => {
            let template = Template::new(std::fs::read_to_string(path)?)?;
            render_md_to_writer(md_file, &template, &mut std::io::stdout())?;
        }
        Args::Serve {} => {}
    }

    Ok(())
}
