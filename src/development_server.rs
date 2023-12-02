use std::convert::Infallible;
use std::ffi::OsStr;
use std::net::SocketAddr;
use std::path::{Path, PathBuf};

use hyper::service::{make_service_fn, service_fn};
use hyper::{Body, Method, Request, Response, Server};
use ramhorns::{Content, Template};
use syntect::parsing::SyntaxSet;

use crate::index::Index;
use crate::post::{ParseError, Post};

impl From<ParseError> for Response<String> {
    fn from(value: ParseError) -> Self {
        let status = if let ParseError::NotFound { .. } = value {
            hyper::StatusCode::NOT_FOUND
        } else {
            hyper::StatusCode::INTERNAL_SERVER_ERROR
        };

        Response::builder()
            .status(status)
            .body(format!("{:?}", value))
            .unwrap()
    }
}

fn template_from_path(path: &Path) -> Result<Template, ParseError> {
    Ok(Template::new(std::fs::read_to_string(path)?)?)
}

fn render_template_to_string<C: Content>(
    template: &Template,
    content: &C,
) -> Result<String, ParseError> {
    let mut buf = Vec::<u8>::new();
    match template.render_to_writer(&mut buf, &content) {
        Ok(()) => String::from_utf8(buf).map_err(|_| ParseError::InternalError),
        Err(err) => Err(err.into()),
    }
}

fn render_page(state: &State, uri_path: &str) -> Response<String> {
    let template = match template_from_path(&state.page_template) {
        Ok(template) => template,
        Err(err) => return err.into(),
    };

    let md_file = {
        let mut md_file = std::path::Path::new(&state.site_root).join(uri_path);

        if md_file.extension().and_then(OsStr::to_str) == Some("html") {
            md_file.set_extension("md");
        }

        md_file
    };

    Post::from_file(&state.site_root, &md_file, &state.syntax_set)
        .and_then(|post| render_template_to_string(&template, &post))
        .and_then(|page| {
            Response::builder()
                .status(hyper::StatusCode::OK)
                .body(page)
                .map_err(|_| ParseError::InternalError)
        })
        .unwrap_or_else(Into::into)
}

fn render_index(state: &State) -> Response<String> {
    let template = match template_from_path(&state.index_template) {
        Ok(template) => template,
        Err(err) => return err.into(),
    };

    let posts_root = {
        let mut path = state.site_root.clone();
        path.push("posts/");
        path
    };
    let content = Index::from_path(&posts_root, &state.syntax_set);

    match content {
        Err(err) => Response::builder()
            .status(hyper::StatusCode::NOT_FOUND)
            .body(format!(
                "Error gathering posts for index from \"{}\": {}",
                posts_root.to_string_lossy(),
                err
            ))
            .unwrap_or_else(|_| ParseError::InternalError.into()),
        Ok(content) => render_template_to_string(&template, &content)
            .and_then(|page| {
                Response::builder()
                    .status(hyper::StatusCode::OK)
                    .body(page)
                    .map_err(|_| ParseError::InternalError)
            })
            .unwrap_or_else(Into::into),
    }
}

async fn build_for_web<'a>(req: Request<Body>, state: &State) -> Response<String> {
    if req.method() != Method::GET {
        return Response::builder()
            .status(hyper::StatusCode::METHOD_NOT_ALLOWED)
            .body("<h1>Method Not Allowed</1>".to_owned())
            .unwrap();
    }

    let uri_path = &req.uri().path()[1..];

    if uri_path.is_empty() || uri_path == "index.html" {
        render_index(state)
    } else {
        render_page(state, uri_path)
    }
}

async fn shutdown_signal() {
    tokio::signal::ctrl_c()
        .await
        .expect("failed to install CTRL+C signal handler");
}

pub async fn serve_forever(
    site_root: PathBuf,
    syntax_set: SyntaxSet,
) -> std::result::Result<(), hyper::Error> {
    let state = {
        let page_template = std::path::Path::new(&site_root).join("_config/layouts/post.hbs");
        let index_template = std::path::Path::new(&site_root).join("_config/layouts/index.hbs");
        std::sync::Arc::new(State {
            site_root,
            page_template,
            index_template,
            syntax_set,
        })
    };

    let make_service = make_service_fn(|_| {
        let state = state.clone();

        let svc_fn = service_fn(move |req| {
            let state = state.clone();
            async move { Ok::<_, Infallible>(build_for_web(req, &state).await) }
        });
        async move { Ok::<_, hyper::Error>(svc_fn) }
    });

    let addr = SocketAddr::from(([127, 0, 0, 1], 1024));
    let server = Server::bind(&addr)
        .serve(make_service)
        .with_graceful_shutdown(shutdown_signal());

    println!("Serving on 127.0.0.1:1024");
    server.await
}

struct State {
    site_root: PathBuf,
    page_template: PathBuf,
    index_template: PathBuf,
    syntax_set: SyntaxSet,
}
