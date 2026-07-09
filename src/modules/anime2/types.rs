use crate::shared::types::entities::anime::HasPoster;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Serialize, Deserialize, Debug, Clone, ToSchema)]
pub struct Anime2Data {
    pub ongoing_anime: Vec<crate::shared::types::entities::anime::OngoingAnimeItem>,
    pub complete_anime: Vec<crate::shared::types::entities::anime::CompleteAnimeItem>,
}

#[derive(Serialize, Deserialize, Debug, Clone, ToSchema)]
pub struct Anime2Response {
    pub status: String,
    pub data: Anime2Data,
}

#[derive(Serialize, Deserialize, Debug, Clone, ToSchema)]
pub struct Genre {
    pub name: String,
    pub slug: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, ToSchema)]
pub struct GenresResponse {
    pub status: String,
    pub data: Vec<Genre>,
}

#[derive(Serialize, Deserialize, Debug, Clone, ToSchema)]
pub struct FiltersApplied {
    pub genre: Option<String>,
    pub status: Option<String>,
    pub r#type: Option<String>,
    pub order: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, ToSchema)]
pub struct FilterResponse {
    pub success: bool,
    pub data: Vec<crate::shared::types::entities::anime::FilterAnimeItem>,
    pub pagination: crate::shared::types::entities::anime::Pagination,
    pub filters_applied: FiltersApplied,
    pub status: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, ToSchema)]
pub struct AnimeDetailData {
    pub title: String,
    pub alternative_title: String,
    pub poster: String,
    pub poster2: String,
    pub r#type: String,
    pub release_date: String,
    pub status: String,
    pub synopsis: String,
    pub studio: String,
    pub genres: Vec<DetailGenre>,
    pub producers: Vec<String>,
    pub recommendations: Vec<Recommendation>,
    pub batch: Vec<DownloadItem>,
    pub ova: Vec<DownloadItem>,
    pub downloads: Vec<DownloadItem>,
}

#[derive(Serialize, Deserialize, Debug, Clone, ToSchema)]
pub struct DetailGenre {
    pub name: String,
    pub slug: String,
    pub anime_url: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, ToSchema)]
pub struct Link {
    pub name: String,
    pub url: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, ToSchema)]
pub struct DownloadItem {
    pub resolution: String,
    pub links: Vec<Link>,
}

#[derive(Serialize, Deserialize, Debug, Clone, ToSchema)]
pub struct Recommendation {
    pub title: String,
    pub slug: String,
    pub poster: String,
    pub status: String,
    pub r#type: String,
}

impl HasPoster for Recommendation {
    fn poster(&self) -> &str {
        &self.poster
    }
    fn set_poster(&mut self, url: String) {
        self.poster = url;
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, ToSchema)]
pub struct DetailResponse {
    pub status: String,
    pub data: AnimeDetailData,
}
