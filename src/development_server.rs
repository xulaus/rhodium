use std::convert::Infallible;
use std::net::SocketAddr;
use std::path::PathBuf;

use hyper::service::{make_service_fn, service_fn};
use hyper::{Body, Method, Request, Response, Server};
use ramhorns::Template;

use crate::post::{ParseError, Post, PostMeta};

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

fn template_from_path(path: &PathBuf) -> Result<Template, ParseError> {
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

    let html = std::ffi::OsString::from("html");
    let md = std::ffi::OsString::from("md");
    let md_file = {
        let mut md_file = std::path::Path::new(&state.site_root).join(uri_path);

        if md_file.extension() == Some(&html) {
            md_file.set_extension(md);
        }

        md_file
    };

    Post::from_file(&state.site_root, &md_file)
        .and_then(|post| render_template_to_string(&template, &post))
        .and_then(|page| {
            Response::builder()
                .status(hyper::StatusCode::OK)
                .body(page)
                .map_err(|_| ParseError::InternalError)
        })
        .unwrap_or_else(Into::into)
}

use ramhorns::Content;
#[derive(Content, Debug)]
pub struct Pagenation {
    first_page: Option<String>,
    previous_page: Option<String>,
    next_page: Option<String>,
    latest_page: Option<String>,
    page: u32,
    total_pages: u32,
}
#[derive(Content, Debug)]
pub struct Index {
    posts: Vec<PostMeta>,
    pagenation: Pagenation,
}

fn render_index(state: &State, uri_path: &str) -> Response<String> {
    let template = match template_from_path(&state.index_template) {
        Ok(template) => template,
        Err(err) => return err.into(),
    };

    fn files_within(path: &PathBuf, acc: &mut Vec<PathBuf>) -> Result<(), std::io::Error> {
        let md = std::ffi::OsString::from("md");
        for entry in std::fs::read_dir(path)? {
            let entry = entry?;
            let metadata = entry.metadata()?;
            let path = entry.path();
            if metadata.is_file() && path.extension() == Some(&md) {
                acc.push(path);
            } else if metadata.is_dir() {
                files_within(&path, acc)?;
            }
        }
        Ok(())
    }

    let posts = {
        let mut posts = vec![];
        let mut path = state.site_root.clone();
        path.push("posts/");
        files_within(&path, &mut posts).expect("Failed to find files");
        posts
    };
    let pagenation = Pagenation {
        first_page: Some("index.html".to_owned()),
        previous_page: None,
        next_page: None,
        latest_page: Some("index.html".to_owned()),
        page: 1,
        total_pages: posts.len() as u32 / 20 + 1,
    };

    let posts = posts
        .iter()
        .filter_map(|path| Post::from_file(&state.site_root, path).ok())
        .map(|p| p.metadata)
        .collect();
    let content = Index { posts, pagenation };

    render_template_to_string(&template, &content)
        .and_then(|page| {
            Response::builder()
                .status(hyper::StatusCode::OK)
                .body(page)
                .map_err(|_| ParseError::InternalError)
        })
        .unwrap_or_else(Into::into)
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
        render_index(state, uri_path)
    } else {
        render_page(state, uri_path)
    }
}

async fn shutdown_signal() {
    tokio::signal::ctrl_c()
        .await
        .expect("failed to install CTRL+C signal handler");
}

pub async fn serve_forever(site_root: PathBuf) -> std::result::Result<(), hyper::Error> {
    let state = {
        let page_template = std::path::Path::new(&site_root).join("page.hbs");
        let index_template = std::path::Path::new(&site_root).join("index.hbs");
        std::sync::Arc::new(State {
            site_root,
            page_template,
            index_template,
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

    server.await
}

struct State {
    site_root: PathBuf,
    page_template: PathBuf,
    index_template: PathBuf,
}
