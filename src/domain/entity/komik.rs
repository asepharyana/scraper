//! Domain entities for komik (comic) data.

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::domain::entity::anime::HasPoster;

#[derive(Serialize, Deserialize, Debug, Clone, ToSchema)]
pub struct KomikGenre {
    pub name: String,
    pub slug: String,
    pub count: Option<u32>,
}

#[derive(Serialize, Deserialize, Debug, Clone, ToSchema)]
pub struct Chapter {
    pub chapter: String,
    pub date: String,
    pub chapter_id: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, ToSchema)]
pub struct ChapterData {
    pub title: String,
    pub next_chapter_id: String,
    pub prev_chapter_id: String,
    pub list_chapter: String,
    pub images: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, ToSchema)]
pub struct DetailData {
    pub title: String,
    pub poster: String,
    pub description: String,
    pub status: String,
    pub r#type: String,
    pub release_date: String,
    pub author: String,
    pub total_chapter: String,
    pub updated_on: String,
    pub genres: Vec<String>,
    pub chapters: Vec<Chapter>,
}

#[derive(Serialize, Deserialize, Debug, Clone, ToSchema)]
pub struct KomikItem {
    pub title: String,
    pub slug: String,
    pub poster: String,
    pub chapter: String,
    pub score: String,
    pub r#type: String,
    pub komik_url: String,
}

impl HasPoster for KomikItem {
    fn poster(&self) -> &str {
        &self.poster
    }
    fn set_poster(&mut self, url: String) {
        self.poster = url;
    }
}
