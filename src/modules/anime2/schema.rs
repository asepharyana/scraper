use serde::Deserialize;
use utoipa::ToSchema;

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct SlugPath {
    pub slug: String,
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct SlugPagePath {
    pub slug: String,
    pub page: u32,
}

#[derive(Deserialize, ToSchema)]
pub struct FilterQuery {
    pub page: Option<u32>,
    pub genre: Option<String>,
    pub status: Option<String>,
    pub r#type: Option<String>,
    pub order: Option<String>,
}

#[derive(Deserialize, ToSchema)]
pub struct GenreQuery {
    pub page: Option<u32>,
    pub status: Option<String>,
    pub order: Option<String>,
}

#[derive(Deserialize, ToSchema)]
pub struct SearchQuery {
    pub q: Option<String>,
}
