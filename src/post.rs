use std::{io, path::PathBuf};

use crate::render::{mdast_into_str_builder, MarkdownError, RenderError, Toc};
use markdown::{mdast, to_mdast, ParseOptions};
use ramhorns::Content;
use thiserror::Error;

#[derive(Content, Debug)]
pub struct Post {
    pub title: String,
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
    pub fn from_file(path: &PathBuf) -> Result<Post, ParseError> {
        let md_string = std::fs::read_to_string(path).map_err(|err| {
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
