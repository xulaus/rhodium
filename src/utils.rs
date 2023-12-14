use std::{path::{Path, PathBuf}, borrow::Cow};
pub fn parameterize(s: &str) -> Cow<str> {
    let mut char_iter = s.chars().enumerate();
    if let Some((i, _)) = char_iter.find(|(_i, x)| !x.is_alphanumeric()) {
        let mut out = String::with_capacity(s.len());
        out += &s[0..i];

        while let Some((start, _)) = char_iter.find(|(_i, x)| x.is_alphanumeric()) {
            out += "-";
            if let Some((end, _)) = char_iter.find(|(_i, x)| !x.is_alphanumeric()) {
                out += &s[start..end];
            } else {
                out += &s[start..];
                break;
            }
        }

        Cow::Owned(out)
    } else {
        Cow::Borrowed(s)
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parameterize() {
        assert_eq!(parameterize("foo"), Cow::Borrowed("foo"));
        assert_eq!(parameterize("foo!!bar"), Cow::<str>::Owned("foo-bar".into()));
        assert_eq!(parameterize("foo!!bar!baz"), Cow::<str>::Owned("foo-bar-baz".into()));
        assert_eq!(parameterize("foo!!bar!baz:"), Cow::<str>::Owned("foo-bar-baz".into()));
    }
}
