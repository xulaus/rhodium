use lazy_static::lazy_static;
use markdown::{mdast, mdast::*};
use std::borrow::Cow;
use syntect::highlighting::{Theme, ThemeSet};
use syntect::html::highlighted_html_for_string;
use syntect::parsing::SyntaxSet;
use thiserror::Error;

use super::utils::parameterize;

#[derive(Error, Debug)]
pub enum RenderError {
    #[error("Error while highlighting {0}")]
    HighlightingError(#[from] syntect::Error),
    #[error("Could not find syntax for {lang}")]
    UnknownLang { lang: String },
    #[error("Header Too Deep")]
    HeaderTooDeep,
    #[error("Rhodium doesn't currently support {node_type}.")]
    NodeNotSupported { node_type: &'static str },
    #[error("Internal Error: md ast nodes have been structured in an unexpected way.")]
    InternalError,
}

lazy_static! {
    static ref THEME_SET: ThemeSet = ThemeSet::load_defaults();
    static ref THEME: &'static Theme = &THEME_SET.themes["base16-ocean.dark"];
}

fn highlight_fragment(s: &str, lang: &str, syntax_set: &SyntaxSet) -> Result<String, RenderError> {
    let syntax = syntax_set
        .syntaxes()
        .iter()
        .find(|&s| s.name.eq_ignore_ascii_case(lang))
        .ok_or_else(|| RenderError::UnknownLang {
            lang: lang.to_owned(),
        })?;

    highlighted_html_for_string(s, syntax_set, syntax, &THEME).map_err(|e| e.into())
}

#[derive(Error, Debug)]
pub enum MarkdownError {
    #[error("Internal Error. Markdown parser started with non root node.")]
    InvalidRoot,
    #[error("Error Parsing Markdown. {wrapped}")]
    ErrorParsing { wrapped: String },
    #[error("First heading in page was not the title. Page should begin with a level 1 heading")]
    FirstHeadingNotTitle,
    #[error("Unable to find page title as the page had no headings. Page should begin with a level 1 heading")]
    NoHeadings,
    #[error(
        "Page should contain only one title (level 1 heading). Second title was {second_title}"
    )]
    ManyTitles { second_title: String },
}

#[derive(Debug)]
pub struct Toc {
    pub depth: u8,
    pub name: String,
    pub children: Vec<Toc>,
}

impl Toc {
    pub fn to_html(&self) -> Option<String> {
        if self.children.is_empty() {
            return None;
        }

        let mut builder = vec![];
        self.child_html_builder(&mut builder);
        Some(builder.concat())
    }

    fn child_html_builder<'a>(&'a self, builder: &mut Vec<Cow<'a, str>>) {
        builder.push(Cow::Borrowed("<ol>"));
        for child in &self.children {
            builder.push(Cow::Borrowed("<li><a href=\"#"));
            builder.push(parameterize(&child.name));
            builder.push(Cow::Borrowed("\">"));
            builder.push(Cow::Borrowed(&child.name));
            builder.push(Cow::Borrowed("</a></li>"));
            if !child.children.is_empty() {
                child.child_html_builder(builder);
            }
        }
        builder.push(Cow::Borrowed("</ol>"));
    }

    pub fn from_mdast(root: &mdast::Root) -> Result<Self, MarkdownError> {
        let mut headings = root.children.iter().filter_map(|node| {
            if let mdast::Node::Heading(heading) = node {
                Some((node, heading))
            } else {
                None
            }
        });
        let (title_node, title) = headings.next().ok_or(MarkdownError::NoHeadings)?;
        if title.depth != 1 {
            return Err(MarkdownError::FirstHeadingNotTitle);
        };

        let mut stack: Vec<Self> = vec![Self {
            depth: title.depth,
            children: vec![],
            name: title_node.to_string(),
        }];

        for (head_node, head) in headings {
            while stack.last().unwrap().depth >= head.depth {
                let child = stack.pop().unwrap();
                let parent = stack.last_mut().ok_or_else(|| MarkdownError::ManyTitles {
                    second_title: head_node.to_string(),
                })?;
                parent.children.push(child);
            }
            stack.push(Self {
                depth: head.depth,
                children: vec![],
                name: head_node.to_string(),
            });
        }
        while stack.len() > 1 {
            let child = stack.pop().unwrap();
            let parent = stack.last_mut().unwrap();
            parent.children.push(child);
        }
        Ok(stack.pop().unwrap())
    }
}

const HEADINGS: [&str; 6] = ["h1", "h2", "h3", "h4", "h5", "h6"];
pub fn mdast_into_str_builder<'a>(
    node: &'a mdast::Node,
    builder: &mut Vec<std::borrow::Cow<'a, str>>,
    syntax_set: &SyntaxSet,
) -> Result<(), RenderError> {
    match node {
        Node::Root(Root { children, .. }) => {
            for child in children {
                mdast_into_str_builder(child, builder, syntax_set)?;
            }
            Ok(())
        }
        Node::Text(Text { value, .. }) | Node::Html(Html { value, .. }) => {
            builder.push(Cow::Borrowed(value));
            Ok(())
        }
        Node::InlineCode(InlineCode { value, .. }) => {
            builder.push(Cow::Borrowed("<code>"));
            builder.push(Cow::Borrowed(value));
            builder.push(Cow::Borrowed("</code>"));
            Ok(())
        }
        Node::Emphasis(Emphasis { children, .. }) => {
            builder.push(Cow::Borrowed("<em>"));
            for child in children {
                mdast_into_str_builder(child, builder, syntax_set)?;
            }
            builder.push(Cow::Borrowed("</em>"));
            Ok(())
        }
        Node::Strong(Strong { children, .. }) => {
            builder.push(Cow::Borrowed("<strong>"));
            for child in children {
                mdast_into_str_builder(child, builder, syntax_set)?;
            }
            builder.push(Cow::Borrowed("</strong>"));
            Ok(())
        }
        Node::Break(Break { .. }) => {
            builder.push(Cow::Borrowed("<br />"));
            Ok(())
        }
        Node::Delete(Delete { children, .. }) => {
            builder.push(Cow::Borrowed("<del>"));
            for child in children {
                mdast_into_str_builder(child, builder, syntax_set)?;
            }
            builder.push(Cow::Borrowed("</del>"));
            Ok(())
        }
        Node::Link(Link {
            children,
            url,
            title,
            ..
        }) => {
            builder.push(Cow::Borrowed("<a href=\""));
            builder.push(Cow::Borrowed(url));
            if let Some(title) = title {
                builder.push(Cow::Borrowed("\" title=\""));
                builder.push(Cow::Borrowed(title));
            }
            builder.push(Cow::Borrowed("\">"));
            for child in children {
                mdast_into_str_builder(child, builder, syntax_set)?;
            }
            builder.push(Cow::Borrowed("</a>"));
            Ok(())
        }
        Node::Code(Code { value, lang, .. }) => {
            if let Some(lang) = lang {
                match highlight_fragment(value, lang, syntax_set) {
                    Ok(highlighted) => {
                        builder.push(Cow::Owned(highlighted));
                        return Ok(());
                    }
                    Err(err) => {
                        eprintln!("Warning! {err}");
                    }
                }
            }

            builder.push(Cow::Borrowed(
                "<pre style=\"background-color:#2b303b;\"><code>",
            ));
            builder.push(Cow::Borrowed(value));
            builder.push(Cow::Borrowed("</code></pre>"));
            Ok(())
        }
        Node::Paragraph(Paragraph { children, .. }) => {
            builder.push(Cow::Borrowed("<p>"));
            for child in children {
                mdast_into_str_builder(child, builder, syntax_set)?;
            }
            builder.push(Cow::Borrowed("</p>"));
            Ok(())
        }
        Node::List(List { children, .. }) => {
            builder.push(Cow::Borrowed("<ol>"));
            for child in children {
                mdast_into_str_builder(child, builder, syntax_set)?;
            }
            builder.push(Cow::Borrowed("</ol>"));
            Ok(())
        }

        Node::BlockQuote(BlockQuote { children, .. }) => {
            builder.push(Cow::Borrowed("<blockquote>"));
            for child in children {
                mdast_into_str_builder(child, builder, syntax_set)?;
            }
            builder.push(Cow::Borrowed("</blockquote>"));
            Ok(())
        }

        Node::Table(Table {
            children, align, ..
        }) => {
            if let Node::TableRow(TableRow {
                children: headers, ..
            }) = &children[0]
            {
                builder.push(Cow::Borrowed("<table><thead><tr>"));
                for (head, align) in headers.iter().zip(align) {
                    match align {
                        AlignKind::None => builder.push(Cow::Borrowed("<th>")),
                        AlignKind::Left => builder.push(Cow::Borrowed("<th align='left'>")),
                        AlignKind::Right => builder.push(Cow::Borrowed("<th align='right'>")),
                        AlignKind::Center => builder.push(Cow::Borrowed("<th align='center'>")),
                    }
                    if let Node::TableCell(TableCell { children: cell, .. }) = head {
                        for node in cell {
                            mdast_into_str_builder(node, builder, syntax_set)?;
                        }
                    } else {
                        return Err(RenderError::InternalError);
                    }
                    builder.push(Cow::Borrowed("</th>"));
                }
                builder.push(Cow::Borrowed("</tr></thead><tbody>"));
                for child in children.iter().skip(1) {
                    if let Node::TableRow(TableRow {
                        children: cells, ..
                    }) = child
                    {
                        for (cell, align) in cells.iter().zip(align) {
                            match align {
                                AlignKind::None => builder.push(Cow::Borrowed("<td>")),
                                AlignKind::Left => builder.push(Cow::Borrowed("<td align='left'>")),
                                AlignKind::Right => {
                                    builder.push(Cow::Borrowed("<td align='right'>"))
                                }
                                AlignKind::Center => {
                                    builder.push(Cow::Borrowed("<td align='center'>"))
                                }
                            }
                            if let Node::TableCell(TableCell { children: cell, .. }) = cell {
                                for node in cell {
                                    mdast_into_str_builder(node, builder, syntax_set)?;
                                }
                            } else {
                                return Err(RenderError::InternalError);
                            }
                            builder.push(Cow::Borrowed("</td>"));
                        }
                    } else {
                        return Err(RenderError::InternalError);
                    }
                }
                builder.push(Cow::Borrowed("</tbody></table>"));
                Ok(())
            } else {
                Err(RenderError::InternalError)
            }
        }

        Node::TableRow(_) | Node::TableCell(_) => Err(RenderError::InternalError),

        Node::ListItem(ListItem { children, .. }) => {
            builder.push(Cow::Borrowed("<li>"));
            for child in children {
                mdast_into_str_builder(child, builder, syntax_set)?;
            }
            builder.push(Cow::Borrowed("</li>"));
            Ok(())
        }
        Node::Heading(Heading {
            depth, children, ..
        }) => {
            let heading = HEADINGS
                .get((*depth - 1) as usize)
                .ok_or(RenderError::HeaderTooDeep)?;
            let name = node.to_string();
            builder.push(Cow::Borrowed("<"));
            builder.push(Cow::Borrowed(heading));
            builder.push(Cow::Borrowed(" id=\""));
            builder.push(Cow::Owned(parameterize(&name).into_owned()));
            builder.push(Cow::Borrowed("\">"));
            for child in children {
                mdast_into_str_builder(child, builder, syntax_set)?;
            }
            builder.push(Cow::Borrowed("</"));
            builder.push(Cow::Borrowed(heading));
            builder.push(Cow::Borrowed(">"));
            Ok(())
        }
        Node::ThematicBreak(ThematicBreak { .. }) => {
            builder.push(Cow::Borrowed("<hr />"));
            Ok(())
        }
        Node::FootnoteReference(FootnoteReference {
            identifier, label, ..
        }) => {
            if let Some(label) = label {
                builder.push(Cow::Borrowed("<sup><a href=\"#"));
                builder.push(parameterize(label));
                builder.push(Cow::Borrowed("\">"));
                builder.push(Cow::Borrowed(identifier));
                builder.push(Cow::Borrowed("</a></sup>"));
            } else {
                builder.push(Cow::Borrowed(identifier));
            }
            Ok(())
        }
        Node::FootnoteDefinition(FootnoteDefinition {
            children,
            identifier,
            label,
            ..
        }) => {
            builder.push(Cow::Borrowed("<div class=\"footnote-definition\" id=\""));
            builder.push(Cow::Borrowed(identifier));
            builder.push(Cow::Borrowed(
                "\"><div class=\"footnote-definition-label\">",
            ));
            builder.push(Cow::Borrowed(label.as_ref().unwrap_or(identifier)));
            builder.push(Cow::Borrowed("</div>"));
            for child in children {
                mdast_into_str_builder(child, builder, syntax_set)?;
            }
            builder.push(Cow::Borrowed("</div>"));
            Ok(())
        }

        // Errors
        Node::Toml(_) | Node::Yaml(_) => {
            // Ignore frontmatter. Where we're going we ain't going to need it.
            Ok(())
        }
        Node::LinkReference(_) => Err(RenderError::NodeNotSupported {
            node_type: "Reference Style Links",
        }),
        Node::MdxjsEsm(_)
        | Node::MdxFlowExpression(_)
        | Node::MdxJsxFlowElement(_)
        | Node::MdxJsxTextElement(_)
        | Node::MdxTextExpression(_) => Err(RenderError::NodeNotSupported { node_type: "JSX" }),

        // TODO:
        Node::Math(_) | Node::InlineMath(_) => {
            Err(RenderError::NodeNotSupported { node_type: "Maths" })
        }
        Node::Definition(_) => Err(RenderError::NodeNotSupported {
            node_type: "Definitions",
        }),

        Node::Image(_) | Node::ImageReference(_) => Err(RenderError::NodeNotSupported {
            node_type: "Images",
        }),
    }
}
