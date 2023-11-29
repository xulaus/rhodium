use crate::post::{Post, PostMeta};
use ramhorns::Content;
use std::ffi::OsStr;
use std::path::PathBuf;

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
    pagenation: Option<Pagenation>,
}

impl Index {
    pub fn from_path(folder: &PathBuf) -> Result<Index, std::io::Error> {
        fn files_within(path: &PathBuf) -> Result<Vec<PathBuf>, std::io::Error> {
            let mut acc = vec![];
            for entry in std::fs::read_dir(path)? {
                let entry = entry?;
                let metadata = entry.metadata()?;
                let path = entry.path();

                if metadata.is_file() && path.extension().and_then(OsStr::to_str) == Some("md") {
                    acc.push(path);
                } else if metadata.is_dir() {
                    acc.extend(files_within(&path)?);
                }
            }
            Ok(acc)
        }

        let posts = files_within(folder)?;
        let pagenation = if posts.len() > 20 {
            Some(Pagenation {
                first_page: Some("index.html".to_owned()),
                previous_page: None,
                next_page: None,
                latest_page: Some("index.html".to_owned()),
                page: 1,
                total_pages: posts.len() as u32 / 20 + 1,
            })
        } else {
            None
        };

        let posts = posts
            .iter()
            .filter_map(|path| Post::from_file(folder, path).ok())
            .map(|p| p.metadata)
            .collect();
        Ok(Index { posts, pagenation })
    }
}
