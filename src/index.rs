use crate::post::{Post, PostMeta};
use crate::utils::files_within;
use ramhorns::Content;
use std::path::{Path, PathBuf};

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
    pub fn from_file_list(site_root: &Path, posts: &[PathBuf]) -> Index {
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
            .filter_map(|path| Post::from_file(site_root, path).ok())
            .map(|p| p.metadata)
            .collect();
        Index { posts, pagenation }
    }
    pub fn from_path(folder: &Path) -> Result<Index, std::io::Error> {
        Ok(Index::from_file_list(folder, &files_within(folder)?))
    }
}
