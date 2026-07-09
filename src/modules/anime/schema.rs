use serde::Deserialize;
use utoipa::ToSchema;

#[derive(Deserialize, ToSchema)]
pub struct SearchQuery {
    pub q: Option<String>,
}

#[derive(Deserialize, ToSchema)]
pub struct SlugPath {
    pub slug: String,
}

#[derive(Deserialize, ToSchema)]
pub struct SlugPagePath {
    pub slug: String,
    pub page: String,
}
