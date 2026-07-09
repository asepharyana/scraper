use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

// Index endpoint types
#[derive(Serialize, Deserialize, Debug, Clone, ToSchema)]
pub struct OngoingAnimeItem {
    pub title: String,
    pub slug: String,
    pub poster: String,
    pub current_episode: String,
    pub anime_url: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, ToSchema)]
pub struct CompleteAnimeItem {
    pub title: String,
    pub slug: String,
    pub poster: String,
    pub episode_count: String,
    pub anime_url: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, ToSchema)]
pub struct AnimeData {
    pub ongoing_anime: Vec<OngoingAnimeItem>,
    pub complete_anime: Vec<CompleteAnimeItem>,
}

// Genre list types
#[derive(Serialize, Deserialize, Debug, Clone, ToSchema)]
pub struct Genre {
    pub name: String,
    pub slug: String,
    pub url: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, ToSchema)]
pub struct GenresResponse {
    pub status: String,
    pub data: Vec<Genre>,
}

// Detail endpoint types
#[derive(Serialize, Deserialize, Debug, Clone, ToSchema)]
pub struct DetailGenre {
    pub name: String,
    pub slug: String,
    pub anime_url: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, ToSchema)]
pub struct EpisodeList {
    pub episode: String,
    pub slug: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, ToSchema)]
pub struct Recommendation {
    pub title: String,
    pub slug: String,
    pub poster: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub r#type: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, ToSchema)]
pub struct AnimeDetailData {
    pub title: String,
    pub alternative_title: String,
    pub poster: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub r#type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
    pub release_date: String,
    pub studio: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub genres: Vec<DetailGenre>,
    pub synopsis: String,
    pub episode_lists: Vec<EpisodeList>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub batch: Vec<EpisodeList>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub producers: Vec<String>,
    pub recommendations: Vec<Recommendation>,
}

#[derive(Serialize, Deserialize, Debug, Clone, ToSchema)]
pub struct DetailResponse {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
    pub data: AnimeDetailData,
}

// Complete anime list types
#[derive(Serialize, Deserialize, Debug, Clone, ToSchema)]
pub struct CompleteAnimeListItem {
    pub title: String,
    pub slug: String,
    pub poster: String,
    pub episode_count: String,
    pub anime_url: String,
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
pub struct ListResponse {
    pub message: String,
    pub data: Vec<CompleteAnimeListItem>,
    pub total: Option<i64>,
    pub pagination: Option<Pagination>,
}

// Full episode types
#[derive(Serialize, Deserialize, Debug, Clone, ToSchema)]
pub struct AnimeInfo {
    pub slug: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, ToSchema)]
pub struct EpisodeInfo {
    pub slug: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, ToSchema)]
pub struct DownloadLink {
    pub server: String,
    pub url: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, ToSchema)]
pub struct AnimeFullData {
    pub episode: String,
    pub episode_number: String,
    pub anime: AnimeInfo,
    pub has_next_episode: bool,
    pub next_episode: Option<EpisodeInfo>,
    pub has_previous_episode: bool,
    pub previous_episode: Option<EpisodeInfo>,
    pub stream_url: String,
    pub download_urls: std::collections::HashMap<String, Vec<DownloadLink>>,
    pub image_url: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, ToSchema)]
pub struct FullResponse {
    pub status: String,
    pub data: AnimeFullData,
}

// Ongoing anime list types
#[derive(Serialize, Deserialize, Debug, Clone, ToSchema)]
pub struct OngoingAnimeListItem {
    pub title: String,
    pub slug: String,
    pub poster: String,
    pub score: String,
    pub anime_url: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, ToSchema)]
pub struct OngoingAnimeResponse {
    pub status: String,
    pub data: Vec<OngoingAnimeListItem>,
    pub pagination: Pagination,
}

// Latest anime types
#[derive(Serialize, Deserialize, Debug, Clone, ToSchema)]
pub struct LatestAnimeItem {
    pub title: String,
    pub slug: String,
    pub poster: String,
    pub episode: String,
    pub anime_url: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, ToSchema)]
pub struct LatestAnimeResponse {
    pub status: String,
    pub data: Vec<LatestAnimeItem>,
    pub pagination: Pagination,
}

// Search types
#[derive(Serialize, Deserialize, Debug, Clone, ToSchema)]
pub struct SearchAnimeItem {
    pub title: String,
    pub slug: String,
    pub poster: String,
    pub episode: String,
    pub anime_url: String,
    pub genres: Vec<String>,
    pub status: String,
    pub rating: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, ToSchema)]
pub struct SearchResponse {
    pub status: String,
    pub data: Vec<SearchAnimeItem>,
    pub pagination: Pagination,
}

// Genre list by slug types
#[derive(Serialize, Deserialize, Debug, Clone, ToSchema)]
pub struct GenreAnimeItem {
    pub title: String,
    pub slug: String,
    pub poster: String,
    pub episode: String,
    pub anime_url: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, ToSchema)]
pub struct GenreListResponse {
    pub status: String,
    pub data: Vec<GenreAnimeItem>,
    pub pagination: Pagination,
}
