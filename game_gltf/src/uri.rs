use std::fmt::{self, Display, Formatter};
use std::path::Path;

#[derive(Clone, Debug, Default)]
pub struct Uri {
    uri: String,
}

impl Uri {
    pub fn push(&mut self, mut path: &str) {
        // "./" is the same as local path.
        if let Some(f) = path.strip_prefix("./") {
            path = f;
        }

        if path.starts_with('/') {
            self.uri = path.to_string();
        } else {
            // Push path to directory path.
            if self.uri.ends_with('/') {
                self.uri.push_str(path);
            } else {
                // Replace final file path.
                // Find the last '/' in the path.
                if let Some((index, _)) =
                    self.uri.bytes().rev().enumerate().find(|(_, c)| *c == b'/')
                {
                    let index = self.uri.len() - index;

                    self.uri.truncate(index);
                    self.uri.push_str(path);
                } else {
                    // The path contains no '/'.
                    self.uri = path.to_owned();
                }
            }
        }
    }

    pub fn as_path(&self) -> &Path {
        self.uri.as_ref()
    }
}

impl Display for Uri {
    #[inline]
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        Display::fmt(&self.uri, f)
    }
}

impl<T> From<T> for Uri
where
    T: AsRef<Path>,
{
    fn from(value: T) -> Self {
        let s = value.as_ref().to_str().unwrap();

        Self { uri: s.to_owned() }
    }
}

#[cfg(test)]
mod tests {
    use super::Uri;

    #[test]
    fn uri_from_str() {
        let path = "test.txt";
        let uri = Uri::from(path);

        assert_eq!(uri.uri, "test.txt");
    }

    #[test]
    fn uri_push_absolute() {
        let mut uri = Uri::from("/base/path");

        uri.push("/new/a");

        assert_eq!(uri.uri, "/new/a");
    }

    #[test]
    fn uri_push_to_directory() {
        let mut uri = Uri::from("/my/directory/");

        uri.push("abc.txt");

        assert_eq!(uri.uri, "/my/directory/abc.txt")
    }

    #[test]
    fn uri_push_replace_file() {
        let mut uri = Uri::from("/my/directory/file");

        uri.push("other");

        assert_eq!(uri.uri, "/my/directory/other");
    }

    #[test]
    fn uri_push_no_directory() {
        let mut uri = Uri::from("file");

        uri.push("other");

        assert_eq!(uri.uri, "other");
    }
}
