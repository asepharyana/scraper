use crate::shared::types::entities::anime::HasPoster;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Serialize, Deserialize, Debug, Clone, ToSchema)]
pub struct Genre {
    pub name: String,
    pub slug: String,
    pub count: Option<u32>,
}

#[derive(Serialize, Deserialize, Debug, Clone, ToSchema)]
pub struct GenresResponse {
    pub status: String,
    pub data: Vec<Genre>,
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
pub struct ChapterResponse {
    pub message: String,
    pub data: ChapterData,
}

#[derive(Serialize, Deserialize, Debug, Clone, ToSchema)]
pub struct Chapter {
    pub chapter: String,
    pub date: String,
    pub chapter_id: String,
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
pub struct DetailResponse {
    pub status: bool,
    pub data: DetailData,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct KomikDetailRequest {
    pub komik_id: String,
    pub chapter_id: Option<String>,
}

#[derive(Debug, Serialize, Clone, ToSchema)]
pub enum KomikDetailEvent {
    Chapter(Chapter),
    Detail(DetailData),
    Error(String),
    EndOfStream,
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

#[derive(Serialize, Deserialize, Debug, Clone, ToSchema)]
pub struct Pagination {
    pub current_page: u32,
    pub last_visible_page: u32,
    pub has_next_page: bool,
    pub next_page: Option<u32>,
    pub has_previous_page: bool,
    pub previous_page: Option<u32>,
}

#[derive(Serialize, Deserialize, Debug, Clone, ToSchema)]
pub struct GenreKomikResponse {
    pub status: String,
    pub genre: String,
    pub data: Vec<KomikItem>,
    pub pagination: Pagination,
}

impl HasPoster for KomikItem {
    fn poster(&self) -> &str {
        &self.poster
    }
    fn set_poster(&mut self, url: String) {
        self.poster = url;
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, ToSchema)]
pub struct SearchKomikResponse {
    pub status: String,
    pub data: Vec<KomikItem>,
    pub pagination: Pagination,
}
