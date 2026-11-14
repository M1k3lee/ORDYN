use miette::SourceSpan;

/// Source code location and span tracking
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Span {
    pub start: usize,
    pub length: usize,
}

impl Span {
    pub const fn new(start: usize, length: usize) -> Self {
        Self { start, length }
    }

    pub fn to_source_span(&self) -> SourceSpan {
        SourceSpan::new(self.start.into(), self.length)
    }

    pub fn merge(self, other: Self) -> Self {
        let start = self.start.min(other.start);
        let end = (self.start + self.length).max(other.start + other.length);
        Self {
            start,
            length: end - start,
        }
    }
}

impl From<Span> for SourceSpan {
    fn from(span: Span) -> Self {
        span.to_source_span()
    }
}
