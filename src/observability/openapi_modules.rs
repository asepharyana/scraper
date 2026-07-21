use utoipa::OpenApi;

/// Manual aggregation of OpenAPI docs from module controllers.
#[derive(OpenApi)]
#[openapi(
    paths(
        // Anime module handlers
        crate::presentation::handler::anime::anime_index,
        crate::presentation::handler::anime::genres,
        crate::presentation::handler::anime::detail_slug,
        crate::presentation::handler::anime::complete_anime_slug,
        crate::presentation::handler::anime::full_slug,
        crate::presentation::handler::anime::ongoing_anime_slug,
        crate::presentation::handler::anime::latest_slug,
        crate::presentation::handler::anime::search_slug_index,
        crate::presentation::handler::anime::search_slug_page,
        crate::presentation::handler::anime::genre_slug_index,
        crate::presentation::handler::anime::genre_slug_page,
        // Anime2 module handlers
        crate::presentation::handler::anime2::index,
        crate::presentation::handler::anime2::genre_list,
        crate::presentation::handler::anime2::filter,
        crate::presentation::handler::anime2::detail_slug,
        crate::presentation::handler::anime2::genre_slug_index,
        crate::presentation::handler::anime2::genre_slug_page,
        crate::presentation::handler::anime2::search_slug_index,
        crate::presentation::handler::anime2::search_slug_page,
        crate::presentation::handler::anime2::latest_slug,
        crate::presentation::handler::anime2::ongoing_anime_slug,
        crate::presentation::handler::anime2::complete_anime_slug,
        // Komik module handlers
        crate::presentation::handler::komik::genre_list,
        crate::presentation::handler::komik::chapter_slug,
        crate::presentation::handler::komik::detail_slug,
        crate::presentation::handler::komik::genre_slug,
        crate::presentation::handler::komik::genre_slug_page,
        crate::presentation::handler::komik::manga_slug,
        crate::presentation::handler::komik::manhua_slug,
        crate::presentation::handler::komik::manhwa_slug,
        crate::presentation::handler::komik::popular_slug,
        crate::presentation::handler::komik::search_slug,
        crate::presentation::handler::komik::search_slug_page,
        // Proxy module handlers
        crate::presentation::handler::proxy::fetch_with_proxy_only,
        crate::presentation::handler::proxy::image_cache,
    ),
    components(
        schemas(
            // Application response wrapper
            crate::presentation::dto::common::ApiResponse<String>,
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
