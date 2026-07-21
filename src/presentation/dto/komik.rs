//! Komik API response DTOs.

use serde::Serialize;
use utoipa::ToSchema;

use crate::domain::entity::anime::Pagination;
use crate::domain::entity::komik::{ChapterData, DetailData, KomikGenre, KomikItem};

#[derive(Serialize, Debug, Clone, ToSchema)]
pub struct GenresResponse {
    pub status: String,
    pub data: Vec<KomikGenre>,
}

#[derive(Serialize, Debug, Clone, ToSchema)]
pub struct GenreKomikResponse {
    pub status: String,
    pub genre: String,
    pub data: Vec<KomikItem>,
    pub pagination: Pagination,
}

#[derive(Serialize, Debug, Clone, ToSchema)]
pub struct DetailResponse {
    pub status: bool,
    pub data: DetailData,
}

#[derive(Serialize, Debug, Clone, ToSchema)]
pub struct ChapterResponse {
    pub message: String,
    pub data: ChapterData,
}

#[derive(Serialize, Debug, Clone, ToSchema)]
pub struct SearchKomikResponse {
    pub status: String,
    pub data: Vec<KomikItem>,
    pub pagination: Pagination,
}
