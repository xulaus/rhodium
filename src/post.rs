use std::{
    io,
    path::{Path, PathBuf},
};

use crate::render::{mdast_into_str_builder, MarkdownError, RenderError, Toc};
use markdown::{mdast, to_mdast, ParseOptions};
use ramhorns::Content;
use syntect::parsing::SyntaxSet;
use thiserror::Error;

#[derive(Content, Debug)]
pub struct PostMeta {
    pub permalink: String,
    pub title: String,
    pub published_date: String,
    pub excerpt: String,
}

#[derive(Content, Debug)]
pub struct Post {
    pub metadata: PostMeta,
    pub toc: Option<String>,
    pub content: String,
}

#[derive(Error, Debug)]
pub enum ParseError {
    #[error("Error Accessing File")]
    FileError {
        #[from]
        source: std::io::Error,
    },
    #[error("Error Parsing Template")]
    CouldntParseMustashe {
        #[from]
        source: ramhorns::Error,
    },
    #[error("Unable to render markdown provided")]
    CouldntRenderMarkdown {
        #[from]
        source: RenderError,
    },
    #[error("Unable to parse markdown provided")]
    CouldntParseMarkdown {
        #[from]
        source: MarkdownError,
    },
    #[error("File not found {file}")]
    NotFound { file: String },
    #[error("Unknown Internal Error")]
    InternalError,
}

impl Post {
    pub fn from_file(site_root: &Path, path: &Path, syntax_set: &SyntaxSet) -> Result<Post, ParseError> {
        let filename = path
            .file_name()
            .and_then(|x| x.to_str())
            .map_or("".to_string(), |x| x.to_string());
        let md_string = std::fs::read_to_string(site_root.join(path)).map_err(|err| {
            if err.kind() == io::ErrorKind::NotFound {
                ParseError::NotFound {
                    file: path.to_string_lossy().to_string(),
                }
            } else {
                err.into()
            }
        })?;

        let md_ast = to_mdast(&md_string, &ParseOptions::gfm())
            .map_err(|err| MarkdownError::ErrorParsing { wrapped: err })?;

        let root = match &md_ast {
            mdast::Node::Root(root) => root,
            _ => {
                return Err(ParseError::CouldntParseMarkdown {
                    source: MarkdownError::InvalidRoot,
                })
            }
        };
        let toc = Toc::from_mdast(root)?;
        let content = {
            let mut builder = vec![];
            mdast_into_str_builder(&md_ast, &mut builder, syntax_set)?;
            builder.concat()
        };

        let new_path = {
            let mut new_path: PathBuf = path.into();
            new_path.set_extension("html");
            new_path.to_string_lossy().to_string()
        };
        let toc_html = toc.to_html();
        let metadata = PostMeta {
            title: toc.name,
            permalink: new_path,
            published_date: filename[0..10].to_string(),
            excerpt: "".to_string(),
        };
        Ok(Post {
            metadata,
            content,
            toc: toc_html,
        })
    }
}
