use utoipa::OpenApi;

/// Manual aggregation of OpenAPI docs from module controllers.
#[derive(OpenApi)]
#[openapi(
    paths(
        // Anime module controllers
        crate::modules::anime::controller::anime_index,
        crate::modules::anime::controller::genres,
        crate::modules::anime::controller::detail_slug,
        crate::modules::anime::controller::complete_anime_slug,
        crate::modules::anime::controller::full_slug,
        crate::modules::anime::controller::ongoing_anime_slug,
        crate::modules::anime::controller::latest_slug,
        crate::modules::anime::controller::search_slug_index,
        crate::modules::anime::controller::search_slug_page,
        crate::modules::anime::controller::genre_slug_index,
        crate::modules::anime::controller::genre_slug_page,
        // Anime2 module controllers
        crate::modules::anime2::controller::index,
        crate::modules::anime2::controller::genre_list,
        crate::modules::anime2::controller::filter,
        crate::modules::anime2::controller::detail_slug,
        crate::modules::anime2::controller::genre_slug_index,
        crate::modules::anime2::controller::genre_slug_page,
        crate::modules::anime2::controller::search_slug_index,
        crate::modules::anime2::controller::search_slug_page,
        crate::modules::anime2::controller::latest_slug,
        crate::modules::anime2::controller::ongoing_anime_slug,
        crate::modules::anime2::controller::complete_anime_slug,
        // Komik module controllers
        crate::modules::komik::controller::genre_list,
        crate::modules::komik::controller::chapter_slug,
        crate::modules::komik::controller::detail_slug,
        crate::modules::komik::controller::genre_slug,
        crate::modules::komik::controller::genre_slug_page,
        crate::modules::komik::controller::manga_slug,
        crate::modules::komik::controller::manhua_slug,
        crate::modules::komik::controller::manhwa_slug,
        crate::modules::komik::controller::popular_slug,
        crate::modules::komik::controller::search_slug,
        crate::modules::komik::controller::search_slug_page,
        // Proxy module controllers
        crate::modules::proxy::controller::fetch_with_proxy_only,
        crate::modules::proxy::controller::image_cache,
    ),
    components(
        schemas(
            // Application response wrapper
            crate::shared::types::ApiResponse<String>,
            // Anime2 types
            crate::modules::anime2::types::Anime2Response,
            crate::modules::anime2::types::GenresResponse,
            crate::modules::anime2::types::FilterResponse,
            crate::modules::anime2::types::DetailResponse,
            // Anime2 query schemas
            crate::modules::anime2::schema::FilterQuery,
            crate::modules::anime2::schema::GenreQuery,
            crate::modules::anime2::schema::SearchQuery,
            // Proxy types
            crate::modules::proxy::types::ImageCacheResponse,
            crate::modules::proxy::types::AuditImageCacheResponse,
            // Proxy request schemas
            crate::modules::proxy::schema::ProxyParams,
            crate::modules::proxy::schema::ImageCacheRequest,
            crate::modules::proxy::schema::AuditImageCacheRequest,
            // Komik types
            crate::modules::komik::types::GenresResponse,
            crate::modules::komik::types::Genre,
            crate::modules::komik::types::ChapterResponse,
            crate::modules::komik::types::DetailResponse,
            crate::modules::komik::types::KomikItem,
            crate::modules::komik::types::Pagination,
            crate::modules::komik::types::GenreKomikResponse,
            crate::modules::komik::types::SearchKomikResponse,
        )
    ),
    tags(
        (name = "anime", description = "Anime endpoints"),
        (name = "anime2", description = "Anime2 endpoints"),
        (name = "komik", description = "Komik endpoints"),
        (name = "proxy", description = "Proxy endpoints"),
    )
)]
pub struct ModuleApiDoc;
