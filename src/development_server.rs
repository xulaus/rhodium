use std::convert::Infallible;
use std::net::SocketAddr;
use std::path::PathBuf;

use hyper::service::{make_service_fn, service_fn};
use hyper::{Body, Method, Request, Response, Server};
use ramhorns::Template;

use crate::post::Post;

fn render_page(state: &State, uri_path: &str)  -> Response<String>{
    let template = std::fs::read_to_string(&state.page_template)
        .map_err(|_| ())
        .and_then(|tpl| Template::new(tpl).map_err(|_| ()));

    let template = if let Ok(template) = template {
        template
    } else {
        return Response::builder()
            .status(hyper::StatusCode::INTERNAL_SERVER_ERROR)
            .body(format!(
                "Couldn't find page template {:?}",
                &state.page_template
            ))
            .unwrap()
    };

    let html: std::ffi::OsString = std::ffi::OsString::from("html");
    let md: std::ffi::OsString = std::ffi::OsString::from("md");
    let md_file = {
        let mut md_file = std::path::Path::new(&state.site_root).join(uri_path);

        if md_file.extension() == Some(&html) {
            md_file.set_extension(md);
        }

        md_file
    };

    let md_string = if let Ok(md_string) = std::fs::read_to_string(&md_file) {
        md_string
    } else {
        return Response::builder()
        .status(hyper::StatusCode::NOT_FOUND)
        .body(format!("Couldn't find {}", md_file.display()))
        .unwrap()
    };


    let mut buf = Vec::<u8>::new();
    if let Ok(post) = Post::from_md_string(&md_string) {
        template.render_to_writer(&mut buf, &post).unwrap()
    } else {
        return Response::builder()
        .status(hyper::StatusCode::INTERNAL_SERVER_ERROR)
        .body("".to_string())
        .unwrap()
    };

    Response::builder()
        .status(hyper::StatusCode::OK)
        .body(String::from_utf8(buf).unwrap())
        .unwrap()
}

use ramhorns::Content;
#[derive(Content, Debug)]
pub struct PostMeta {
}
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
    pages: Vec<PostMeta>,
    pagenation: Pagenation
}

fn render_index(state: &State, uri_path: &str)  -> Response<String>{
    let template = std::fs::read_to_string(&state.index_template)
        .map_err(|_| ())
        .and_then(|tpl| Template::new(tpl).map_err(|_| ()));

    let template = if let Ok(template) = template {
        template
    } else {
        return Response::builder()
            .status(hyper::StatusCode::INTERNAL_SERVER_ERROR)
            .body(format!(
                "Couldn't find index template {:?}",
                &state.index_template
            ))
            .unwrap()
    };
    let mut buf: Vec<u8> = Vec::<u8>::new();

    let pagenation = Pagenation {
        first_page: Some("index.html".to_owned()),
        previous_page: None,
        next_page: None,
        latest_page: Some("index.html".to_owned()),
        page: 1,
        total_pages: 1
    };
    let content = Index{pages:vec![], pagenation};
    match template.render_to_writer(&mut buf, &content) {
        Err(err) => Response::builder()
                .status(hyper::StatusCode::INTERNAL_SERVER_ERROR)
                .body(err.to_string())
                .unwrap(),
        Ok(()) =>
            Response::builder()
                .status(hyper::StatusCode::OK)
                .body(String::from_utf8(buf).unwrap())
                .unwrap()
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
            index_template
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
    index_template: PathBuf
}
