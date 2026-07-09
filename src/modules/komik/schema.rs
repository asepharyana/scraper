use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct SlugPath {
    pub slug: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SlugPagePath {
    pub slug: String,
    pub page: String,
}

#[derive(Deserialize)]
pub struct ChapterQuery {
    /// URL-friendly identifier for the chapter (typically the chapter slug or URL path)
    pub chapter_url: Option<String>,
}
