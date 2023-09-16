use crate::render::{mdast_into_str_builder, MarkdownError, RenderError, Toc};
use markdown::{mdast, to_mdast, ParseOptions};
use ramhorns::Content;

#[derive(Content, Debug)]
pub struct Post {
    pub title: String,
    pub toc: Option<String>,
    pub content: String,
}

impl Post {
    pub fn from_md_string(md_string: &str) -> Result<Post, RenderError> {
        let md_ast = to_mdast(md_string, &ParseOptions::gfm())
            .map_err(|err| MarkdownError::ErrorParsing { wrapped: err })?;

        let root = match &md_ast {
            mdast::Node::Root(root) => root,
            _ => {
                return Err(RenderError::ErrorParsing {
                    source: MarkdownError::InvalidRoot,
                })
            }
        };
        let toc = Toc::from_mdast(root)?;
        let content = {
            let mut builder = vec![];
            mdast_into_str_builder(&md_ast, &mut builder)?;
            builder.concat()
        };

        let toc_html = toc.to_html();
        Ok(Post {
            title: toc.name,
            content,
            toc: toc_html,
        })
    }
}
