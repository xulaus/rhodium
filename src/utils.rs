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
