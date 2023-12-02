use std::path::{Path, PathBuf};

struct NonAlphaNumRunSearcher<'a> {
    haystack: &'a str,
    cur: usize,
}
unsafe impl<'a> std::str::pattern::Searcher<'a> for NonAlphaNumRunSearcher<'a> {
    fn haystack(&self) -> &'a str {
        self.haystack
    }
    fn next(&mut self) -> std::str::pattern::SearchStep {
        use std::str::pattern::SearchStep;
        let mut rest = self.haystack.chars().enumerate().skip(self.cur);
        if let Some((begin, first)) = rest.next() {
            if first.is_alphanumeric() {
                (self.cur, _) = rest
                    .find(|(_i, x)| !x.is_alphanumeric())
                    .unwrap_or((self.haystack.len(), ' '));
                SearchStep::Reject(begin, self.cur)
            } else {
                (self.cur, _) = rest
                    .find(|(_i, x)| x.is_alphanumeric())
                    .unwrap_or((self.haystack.len(), ' '));
                SearchStep::Match(begin, self.cur)
            }
        } else {
            SearchStep::Done
        }
    }
}

struct NonAlphaNumRun {}

impl<'a> std::str::pattern::Pattern<'a> for NonAlphaNumRun {
    type Searcher = NonAlphaNumRunSearcher<'a>;

    fn into_searcher(self, haystack: &'a str) -> Self::Searcher {
        NonAlphaNumRunSearcher::<'a> { haystack, cur: 0 }
    }
}

pub fn parameterize(s: &str) -> String {
    s.replace(NonAlphaNumRun {}, "-").to_lowercase()
}

pub fn files_within(path: &Path) -> Result<Vec<PathBuf>, std::io::Error> {
    let mut acc = vec![];

    fn inner(root: &Path, branch: &Path, acc: &mut Vec<PathBuf>) -> Result<(), std::io::Error> {
        for entry in std::fs::read_dir(root.join(branch))? {
            let entry = entry?;
            let metadata = entry.metadata()?;
            let file_name = entry.file_name();
            let path = PathBuf::from(&file_name);

            if metadata.is_file()
                && path.extension().and_then(std::ffi::OsStr::to_str) == Some("md")
            {
                acc.push(branch.join(file_name));
            } else if metadata.is_dir() {
                inner(root, &branch.join(file_name), acc)?;
            }
        }
        Ok(())
    }

    inner(path, &PathBuf::new(), &mut acc)?;
    Ok(acc)
}
