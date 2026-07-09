use crate::shared::errors::AppError;
use crate::shared::utils::parse_html;
use crate::shared::utils::scraping::{attr, extract_slug, selector, text, text_from_or};
use once_cell::sync::Lazy;
use regex::Regex;
use scraper::Selector;

static ITEM_SELECTOR: Lazy<Selector> = Lazy::new(|| Selector::parse("article.bs").unwrap());
static TITLE_SELECTOR: Lazy<Selector> = Lazy::new(|| Selector::parse(".tt h2").unwrap());
static IMG_SELECTOR: Lazy<Selector> = Lazy::new(|| Selector::parse("img").unwrap());
static SCORE_SELECTOR: Lazy<Selector> = Lazy::new(|| Selector::parse(".numscore").unwrap());
static STATUS_SELECTOR: Lazy<Selector> = Lazy::new(|| Selector::parse(".status").unwrap());
static TYPE_SELECTOR: Lazy<Selector> = Lazy::new(|| Selector::parse(".type").unwrap());
static LINK_SELECTOR: Lazy<Selector> = Lazy::new(|| Selector::parse("a").unwrap());
static PAGINATION_SELECTOR: Lazy<Selector> =
    Lazy::new(|| Selector::parse(".pagination .page-numbers:not(.next)").unwrap());
static NEXT_SELECTOR: Lazy<Selector> = Lazy::new(|| Selector::parse(".pagination .next").unwrap());
static SLUG_REGEX: Lazy<Regex> = Lazy::new(|| Regex::new(r"/([^/]+)/?$").unwrap());
static GENRE_SLUG_REGEX: Lazy<Regex> = Lazy::new(|| Regex::new(r"genre-(.+)$").unwrap());
pub fn parse_ongoing_anime(
    html: &str,
) -> Result<Vec<crate::shared::types::entities::anime::OngoingAnimeItem>, AppError> {
    let items = parse_ongoing_anime_with_score(html)?;
    Ok(items
        .into_iter()
        .map(
            |item| crate::shared::types::entities::anime::OngoingAnimeItem {
                title: item.title,
                slug: item.slug,
                poster: item.poster,
                current_episode: item.score,
                anime_url: item.anime_url,
            },
        )
        .collect())
}

pub fn parse_complete_anime(
    html: &str,
) -> Result<Vec<crate::shared::types::entities::anime::CompleteAnimeItem>, AppError> {
    let document = parse_html(html);
    let mut complete_anime = Vec::new();

    for element in document.select(&ITEM_SELECTOR) {
        let title = element
            .select(&TITLE_SELECTOR)
            .next()
            .map(|e| e.text().collect::<String>().trim().to_string())
            .unwrap_or_default();
        let poster = element
            .select(&IMG_SELECTOR)
            .next()
            .and_then(|e| e.value().attr("src").or(e.value().attr("data-src")))
            .unwrap_or("")
            .to_string();
        let anime_url = element
            .select(&LINK_SELECTOR)
            .next()
            .and_then(|e| e.value().attr("href"))
            .unwrap_or("")
            .to_string();
        let slug = SLUG_REGEX
            .captures(&anime_url)
            .and_then(|cap| cap.get(1))
            .map(|m| m.as_str())
            .unwrap_or("")
            .to_string();
        let episode_count = text_from_or(&element, &STATUS_SELECTOR, "N/A");

        if !title.is_empty() {
            complete_anime.push(crate::shared::types::entities::anime::CompleteAnimeItem {
                title,
                slug,
                poster,
                episode_count,
                anime_url,
            });
        }
    }

    Ok(complete_anime)
}

pub fn parse_genres(html: &str) -> Result<Vec<crate::modules::anime2::types::Genre>, AppError> {
    let document = parse_html(html);
    let mut genres = Vec::new();
    let genre_label_selector = selector("label[for^=\"genre-\"]").ok_or_else(|| {
        AppError::ScraperError("Invalid selector: label[for^=\"genre-\"]".to_string())
    })?;

    for element in document.select(&genre_label_selector) {
        let name = text(&element).trim().to_string();
        let for_attr = attr(&element, "for").unwrap_or_default();

        let slug = GENRE_SLUG_REGEX
            .captures(&for_attr)
            .and_then(|cap| cap.get(1))
            .map(|m| m.as_str())
            .unwrap_or("")
            .to_string();

        if !name.is_empty() && !slug.is_empty() {
            genres.push(crate::modules::anime2::types::Genre { name, slug });
        }
    }

    Ok(genres)
}

pub fn parse_filter_page(
    html: &str,
    current_page: u32,
) -> Result<
    (
        Vec<crate::shared::types::entities::anime::FilterAnimeItem>,
        crate::shared::types::entities::anime::Pagination,
    ),
    AppError,
> {
    let document = parse_html(html);
    let mut anime_list = Vec::new();

    for element in document.select(&ITEM_SELECTOR) {
        let title = element
            .select(&TITLE_SELECTOR)
            .next()
            .map(|e| e.text().collect::<String>().trim().to_string())
            .unwrap_or_default();

        let poster = element
            .select(&IMG_SELECTOR)
            .next()
            .and_then(|e| e.value().attr("src").or(e.value().attr("data-src")))
            .unwrap_or("")
            .to_string();

        let score = element
            .select(&SCORE_SELECTOR)
            .next()
            .map(|e| e.text().collect::<String>().trim().to_string())
            .unwrap_or("N/A".to_string());

        let status = element
            .select(&STATUS_SELECTOR)
            .next()
            .map(|e| e.text().collect::<String>().trim().to_string())
            .unwrap_or("Unknown".to_string());

        let anime_type = element
            .select(&TYPE_SELECTOR)
            .next()
            .map(|e| e.text().collect::<String>().trim().to_string())
            .unwrap_or("Unknown".to_string());

        let anime_url = element
            .select(&LINK_SELECTOR)
            .next()
            .and_then(|e| e.value().attr("href"))
            .unwrap_or("")
            .to_string();

        let slug = SLUG_REGEX
            .captures(&anime_url)
            .and_then(|cap| cap.get(1))
            .map(|m| m.as_str())
            .unwrap_or("")
            .to_string();

        if !title.is_empty() {
            anime_list.push(crate::shared::types::entities::anime::FilterAnimeItem {
                title,
                slug,
                poster,
                score,
                status,
                r#type: anime_type,
                anime_url,
            });
        }
    }

    let last_visible_page = document
        .select(&PAGINATION_SELECTOR)
        .next_back()
        .map(|e| {
            e.text()
                .collect::<String>()
                .trim()
                .parse::<u32>()
                .unwrap_or(1)
        })
        .unwrap_or(1);

    let has_next_page = document.select(&NEXT_SELECTOR).next().is_some();
    let pagination = crate::shared::types::entities::anime::Pagination {
        current_page,
        last_visible_page,
        has_next_page,
        next_page: if has_next_page {
            Some(current_page + 1)
        } else {
            None
        },
        has_previous_page: current_page > 1,
        previous_page: if current_page > 1 {
            Some(current_page - 1)
        } else {
            None
        },
    };

    Ok((anime_list, pagination))
}

pub fn parse_genre_anime(
    html: &str,
) -> Result<Vec<crate::shared::types::entities::anime::GenreAnimeItem>, AppError> {
    let document = parse_html(html);
    let mut anime_list = Vec::new();

    for element in document.select(&ITEM_SELECTOR) {
        let title = element
            .select(&TITLE_SELECTOR)
            .next()
            .map(|e| e.text().collect::<String>().trim().to_string())
            .unwrap_or_default();

        let poster = element
            .select(&IMG_SELECTOR)
            .next()
            .and_then(|e| e.value().attr("src").or(e.value().attr("data-src")))
            .unwrap_or("")
            .to_string();

        let score = element
            .select(&SCORE_SELECTOR)
            .next()
            .map(|e| e.text().collect::<String>().trim().to_string())
            .unwrap_or("N/A".to_string());

        let status = element
            .select(&STATUS_SELECTOR)
            .next()
            .map(|e| e.text().collect::<String>().trim().to_string())
            .unwrap_or("Unknown".to_string());

        let anime_url = element
            .select(&LINK_SELECTOR)
            .next()
            .and_then(|e| e.value().attr("href"))
            .unwrap_or("")
            .to_string();

        let slug = SLUG_REGEX
            .captures(&anime_url)
            .and_then(|cap| cap.get(1))
            .map(|m| m.as_str())
            .unwrap_or("")
            .to_string();

        if !title.is_empty() {
            anime_list.push(crate::shared::types::entities::anime::GenreAnimeItem {
                title,
                slug,
                poster,
                score,
                status,
                anime_url,
            });
        }
    }

    Ok(anime_list)
}

pub fn parse_search_anime(
    html: &str,
) -> Result<Vec<crate::shared::types::entities::anime::SearchAnimeItem>, AppError> {
    let document = parse_html(html);
    let mut anime_list = Vec::new();

    for element in document.select(&ITEM_SELECTOR) {
        let title = element
            .select(&TITLE_SELECTOR)
            .next()
            .map(|e| e.text().collect::<String>().trim().to_string())
            .unwrap_or_default();

        let poster = element
            .select(&IMG_SELECTOR)
            .next()
            .and_then(|e| e.value().attr("src").or(e.value().attr("data-src")))
            .unwrap_or("")
            .to_string();

        let anime_url = element
            .select(&LINK_SELECTOR)
            .next()
            .and_then(|e| e.value().attr("href"))
            .unwrap_or("")
            .to_string();

        let slug = SLUG_REGEX
            .captures(&anime_url)
            .and_then(|cap| cap.get(1))
            .map(|m| m.as_str())
            .unwrap_or("")
            .to_string();

        if !title.is_empty() {
            anime_list.push(crate::shared::types::entities::anime::SearchAnimeItem {
                title,
                slug,
                poster,
                description: String::new(),
                anime_url,
                genres: Vec::new(),
                rating: "N/A".to_string(),
                r#type: "Unknown".to_string(),
                season: "Unknown".to_string(),
            });
        }
    }

    Ok(anime_list)
}

pub fn parse_latest_anime(
    html: &str,
) -> Result<Vec<crate::shared::types::entities::anime::LatestAnimeItem>, AppError> {
    let document = parse_html(html);
    let mut anime_list = Vec::new();

    for element in document.select(&ITEM_SELECTOR) {
        let title = element
            .select(&TITLE_SELECTOR)
            .next()
            .map(|e| e.text().collect::<String>().trim().to_string())
            .unwrap_or_default();

        let poster = element
            .select(&IMG_SELECTOR)
            .next()
            .and_then(|e| e.value().attr("src").or(e.value().attr("data-src")))
            .unwrap_or("")
            .to_string();

        let score = element
            .select(&SCORE_SELECTOR)
            .next()
            .map(|e| e.text().collect::<String>().trim().to_string())
            .unwrap_or("N/A".to_string());

        let anime_url = element
            .select(&LINK_SELECTOR)
            .next()
            .and_then(|e| e.value().attr("href"))
            .unwrap_or("")
            .to_string();

        let slug = SLUG_REGEX
            .captures(&anime_url)
            .and_then(|cap| cap.get(1))
            .map(|m| m.as_str())
            .unwrap_or("")
            .to_string();

        if !title.is_empty() {
            anime_list.push(crate::shared::types::entities::anime::LatestAnimeItem {
                title,
                slug,
                poster,
                current_episode: "N/A".to_string(),
                score,
                anime_url,
            });
        }
    }

    Ok(anime_list)
}

pub fn parse_ongoing_anime_with_score(
    html: &str,
) -> Result<Vec<crate::shared::types::entities::anime::OngoingAnimeItemWithScore>, AppError> {
    let document = parse_html(html);
    let mut anime_list = Vec::new();

    for element in document.select(&ITEM_SELECTOR) {
        let title = element
            .select(&TITLE_SELECTOR)
            .next()
            .map(|e| e.text().collect::<String>().trim().to_string())
            .unwrap_or_default();

        let poster = element
            .select(&IMG_SELECTOR)
            .next()
            .and_then(|e| e.value().attr("src").or(e.value().attr("data-src")))
            .unwrap_or("")
            .to_string();

        let score = element
            .select(&SCORE_SELECTOR)
            .next()
            .map(|e| e.text().collect::<String>().trim().to_string())
            .unwrap_or("N/A".to_string());

        let anime_url = element
            .select(&LINK_SELECTOR)
            .next()
            .and_then(|e| e.value().attr("href"))
            .unwrap_or("")
            .to_string();

        let slug = SLUG_REGEX
            .captures(&anime_url)
            .and_then(|cap| cap.get(1))
            .map(|m| m.as_str())
            .unwrap_or("")
            .to_string();

        if !title.is_empty() {
            anime_list.push(
                crate::shared::types::entities::anime::OngoingAnimeItemWithScore {
                    title,
                    slug,
                    poster,
                    score,
                    anime_url,
                },
            );
        }
    }

    Ok(anime_list)
}

pub fn parse_pagination(
    document: &scraper::Html,
    current_page: u32,
) -> Result<crate::shared::types::entities::anime::Pagination, String> {
    let last_visible_page = document
        .select(&PAGINATION_SELECTOR)
        .next_back()
        .map(|e| {
            e.text()
                .collect::<String>()
                .trim()
                .parse::<u32>()
                .unwrap_or(1)
        })
        .unwrap_or(1);

    let has_next_page = document.select(&NEXT_SELECTOR).next().is_some();
    let pagination = crate::shared::types::entities::anime::Pagination {
        current_page,
        last_visible_page,
        has_next_page,
        next_page: if has_next_page {
            Some(current_page + 1)
        } else {
            None
        },
        has_previous_page: current_page > 1,
        previous_page: if current_page > 1 {
            Some(current_page - 1)
        } else {
            None
        },
    };

    Ok(pagination)
}

pub fn parse_pagination_with_string(
    document: &scraper::Html,
    current_page: u32,
) -> Result<crate::shared::types::entities::anime::PaginationWithStringPages, String> {
    let last_visible_page = document
        .select(&PAGINATION_SELECTOR)
        .next_back()
        .map(|e| {
            e.text()
                .collect::<String>()
                .trim()
                .parse::<u32>()
                .unwrap_or(1)
        })
        .unwrap_or(1);

    let has_next_page = document.select(&NEXT_SELECTOR).next().is_some();
    let pagination = crate::shared::types::entities::anime::PaginationWithStringPages {
        current_page,
        last_visible_page,
        has_next_page,
        next_page: if has_next_page {
            Some((current_page + 1).to_string())
        } else {
            None
        },
        has_previous_page: current_page > 1,
        previous_page: if current_page > 1 {
            Some((current_page - 1).to_string())
        } else {
            None
        },
    };

    Ok(pagination)
}

pub fn parse_anime_detail(
    html: &str,
) -> Result<crate::modules::anime2::types::AnimeDetailData, AppError> {
    let document = parse_html(html);

    let title_selector = selector(".entry-title")
        .ok_or_else(|| AppError::ScraperError("Invalid selector: .entry-title".to_string()))?;
    let alt_title_selector = selector(".alter")
        .ok_or_else(|| AppError::ScraperError("Invalid selector: .alter".to_string()))?;
    let poster_selector = selector(".thumb img, .thumbook img, .wp-post-image, .ts-post-image")
        .ok_or_else(|| {
            AppError::ScraperError(
                "Invalid selector: .thumb img, .thumbook img, .wp-post-image, .ts-post-image"
                    .to_string(),
            )
        })?;
    let poster2_selector = selector(".bigcover img, .bixbox.animefull .bigcover .ime img")
        .ok_or_else(|| {
            AppError::ScraperError(
                "Invalid selector: .bigcover img, .bixbox.animefull .bigcover .ime img".to_string(),
            )
        })?;
    let spe_span_selector = selector(".info-content .spe span").ok_or_else(|| {
        AppError::ScraperError("Invalid selector: .info-content .spe span".to_string())
    })?;
    let a_selector =
        selector("a").ok_or_else(|| AppError::ScraperError("Invalid selector: a".to_string()))?;
    let synopsis_selector = selector(".entry-content p")
        .ok_or_else(|| AppError::ScraperError("Invalid selector: .entry-content p".to_string()))?;
    let genre_selector = selector(".genxed a")
        .ok_or_else(|| AppError::ScraperError("Invalid selector: .genxed a".to_string()))?;
    let download_container_selector = selector(".soraddl.dlone")
        .ok_or_else(|| AppError::ScraperError("Invalid selector: .soraddl.dlone".to_string()))?;
    let resolution_selector = selector(".res")
        .ok_or_else(|| AppError::ScraperError("Invalid selector: .res".to_string()))?;
    let link_selector = selector(".slink a")
        .ok_or_else(|| AppError::ScraperError("Invalid selector: .slink a".to_string()))?;
    let h3_selector =
        selector("h3").ok_or_else(|| AppError::ScraperError("Invalid selector: h3".to_string()))?;
    let recommendation_selector = selector(".listupd .bs")
        .ok_or_else(|| AppError::ScraperError("Invalid selector: .listupd .bs".to_string()))?;
    let rec_title_selector = selector(".ntitle")
        .ok_or_else(|| AppError::ScraperError("Invalid selector: .ntitle".to_string()))?;
    let rec_img_selector = selector("img")
        .ok_or_else(|| AppError::ScraperError("Invalid selector: img".to_string()))?;
    let status_selector = selector(".status")
        .ok_or_else(|| AppError::ScraperError("Invalid selector: .status".to_string()))?;
    let type_selector = selector(".typez")
        .ok_or_else(|| AppError::ScraperError("Invalid selector: .typez".to_string()))?;

    let title = text_from_or(&document.root_element(), &title_selector, "");
    let alternative_title = text_from_or(&document.root_element(), &alt_title_selector, "");

    let poster = document
        .select(&poster_selector)
        .next()
        .and_then(|e| {
            attr(&e, "src")
                .or_else(|| attr(&e, "data-src"))
                .or_else(|| attr(&e, "data-lazy-src"))
        })
        .unwrap_or_default();

    let poster2 = document
        .select(&poster2_selector)
        .next()
        .and_then(|e| {
            attr(&e, "src")
                .or_else(|| attr(&e, "data-src"))
                .or_else(|| attr(&e, "data-lazy-src"))
        })
        .unwrap_or_default();

    let r#type = document
        .select(&spe_span_selector)
        .find(|e| text(&e).contains("Tipe:"))
        .and_then(|span| span.select(&a_selector).next())
        .map(|e| text(&e))
        .unwrap_or_default();

    let release_date = document
        .select(&spe_span_selector)
        .find(|e| text(&e).contains("Dirilis:"))
        .map(|e| text(&e))
        .unwrap_or_default();

    let status = document
        .select(&spe_span_selector)
        .find(|e| text(&e).contains("Status:"))
        .map(|e| text(&e))
        .unwrap_or_default();

    let synopsis = text_from_or(&document.root_element(), &synopsis_selector, "");

    let studio = document
        .select(&spe_span_selector)
        .find(|e| text(&e).contains("Studio:"))
        .and_then(|span| span.select(&a_selector).next())
        .map(|e| text(&e))
        .unwrap_or_default();

    let mut genres = Vec::new();
    for element in document.select(&genre_selector) {
        let name = text(&element);
        let anime_url = attr(&element, "href").unwrap_or_default();
        let genre_slug = extract_slug(&anime_url);
        genres.push(crate::modules::anime2::types::DetailGenre {
            name,
            slug: genre_slug,
            anime_url,
        });
    }

    let mut batch = Vec::new();
    let mut ova = Vec::new();
    let mut downloads = Vec::new();

    for element in document.select(&download_container_selector) {
        let title = element
            .select(&h3_selector)
            .next()
            .map(|e| text(&e))
            .unwrap_or_else(|| "Unknown".to_string());

        let category = title.to_lowercase();
        let is_batch = category.contains("batch");
        let is_ova = category.contains("ova");

        let mut all_links = Vec::new();

        let row_selector = selector("table tr")
            .ok_or_else(|| AppError::ScraperError("Invalid selector: table tr".to_string()))?;
        for row in element.select(&row_selector) {
            let resolution = text_from_or(&row, &resolution_selector, "");

            for link_element in row.select(&link_selector) {
                let provider = text(&link_element);
                let url = attr(&link_element, "href").unwrap_or_default();

                let name = if !resolution.is_empty() {
                    format!("{} - {}", resolution, provider)
                } else {
                    provider
                };

                all_links.push(crate::modules::anime2::types::Link { name, url });
            }
        }

        let download_item = crate::modules::anime2::types::DownloadItem {
            resolution: title,
            links: all_links,
        };

        if is_batch {
            batch.push(download_item);
        } else if is_ova {
            ova.push(download_item);
        } else {
            downloads.push(download_item);
        }
    }

    let mut recommendations = Vec::new();
    for element in document.select(&recommendation_selector) {
        let title = text_from_or(&element, &rec_title_selector, "");

        let anime_url = element
            .select(&a_selector)
            .next()
            .and_then(|e| attr(&e, "href"))
            .unwrap_or_default();

        let rec_slug = extract_slug(&anime_url);

        let poster = element
            .select(&rec_img_selector)
            .next()
            .and_then(|e| attr(&e, "data-src").or_else(|| attr(&e, "src")))
            .unwrap_or_default();

        let status = text_from_or(&element, &status_selector, "");

        let r#type = text_from_or(&element, &type_selector, "");

        recommendations.push(crate::modules::anime2::types::Recommendation {
            title,
            slug: rec_slug,
            poster,
            status,
            r#type,
        });
    }

    Ok(crate::modules::anime2::types::AnimeDetailData {
        title,
        alternative_title,
        poster,
        poster2,
        r#type,
        release_date,
        status,
        synopsis,
        studio,
        genres,
        producers: vec![],
        recommendations,
        batch,
        ova,
        downloads,
    })
}

pub fn parse_genre_page(
    html: &str,
    current_page: u32,
) -> Result<
    (
        Vec<crate::shared::types::entities::anime::GenreAnimeItem>,
        crate::shared::types::entities::anime::Pagination,
    ),
    AppError,
> {
    let document = parse_html(html);
    let anime_list = parse_genre_anime(html)?;
    let pagination = parse_pagination(&document, current_page)?;
    Ok((anime_list, pagination))
}

pub fn parse_search_page(
    html: &str,
    current_page: u32,
) -> Result<
    (
        Vec<crate::shared::types::entities::anime::SearchAnimeItem>,
        crate::shared::types::entities::anime::PaginationWithStringPages,
    ),
    AppError,
> {
    let document = parse_html(html);
    let data = parse_search_anime(html)?;
    let pagination = parse_pagination_with_string(&document, current_page)?;
    Ok((data, pagination))
}

pub fn parse_latest_page(
    html: &str,
    current_page: u32,
) -> Result<
    (
        Vec<crate::shared::types::entities::anime::LatestAnimeItem>,
        crate::shared::types::entities::anime::Pagination,
    ),
    AppError,
> {
    let document = parse_html(html);
    let anime_list = parse_latest_anime(html)?;
    let pagination = parse_pagination(&document, current_page)?;
    Ok((anime_list, pagination))
}

pub fn parse_ongoing_page(
    html: &str,
    current_page: u32,
) -> Result<
    (
        Vec<crate::shared::types::entities::anime::OngoingAnimeItemWithScore>,
        crate::shared::types::entities::anime::Pagination,
    ),
    AppError,
> {
    let document = parse_html(html);
    let anime_list = parse_ongoing_anime_with_score(html)?;
    let pagination = parse_pagination(&document, current_page)?;
    Ok((anime_list, pagination))
}

pub fn parse_complete_page(
    html: &str,
    current_page: u32,
) -> Result<
    (
        Vec<crate::shared::types::entities::anime::CompleteAnimeItem>,
        crate::shared::types::entities::anime::Pagination,
    ),
    AppError,
> {
    let document = parse_html(html);
    let anime_list = parse_complete_anime(html)?;
    let pagination = parse_pagination(&document, current_page)?;
    Ok((anime_list, pagination))
}
