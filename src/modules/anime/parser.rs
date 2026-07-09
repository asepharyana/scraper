use crate::modules::anime::types::*;
use crate::shared::errors::AppError;
use crate::shared::utils::parse_html;
use crate::shared::utils::scraping::{
    attr, attr_from, attr_from_or, extract_slug, selector, text, text_from_or,
};

pub fn parse_ongoing_anime(html: &str) -> Result<Vec<OngoingAnimeItem>, AppError> {
    let document = parse_html(html);
    let mut ongoing_anime = Vec::new();

    let venz_selector = selector(".venz ul li").unwrap();
    let title_selector = selector(".thumbz h2.jdlflm").unwrap();
    let link_selector = selector("a").unwrap();
    let img_selector = selector("img").unwrap();
    let episode_selector = selector(".epz").unwrap();

    for element in document.select(&venz_selector) {
        let title = text_from_or(&element, &title_selector, "");
        let href = attr_from(&element, &link_selector, "href").unwrap_or_default();
        let slug = extract_slug(&href);
        let poster = attr_from_or(&element, &img_selector, "src", "");
        let current_episode = text_from_or(&element, &episode_selector, "N/A");
        let anime_url = attr_from_or(&element, &link_selector, "href", "");

        if !title.is_empty() {
            ongoing_anime.push(OngoingAnimeItem {
                title,
                slug,
                poster,
                current_episode,
                anime_url,
            });
        }
    }
    Ok(ongoing_anime)
}

pub fn parse_complete_anime(html: &str) -> Result<Vec<CompleteAnimeItem>, AppError> {
    let document = parse_html(html);
    let mut complete_anime = Vec::new();

    let venz_selector = selector(".venz ul li").unwrap();
    let title_selector = selector(".thumbz h2.jdlflm").unwrap();
    let link_selector = selector("a").unwrap();
    let img_selector = selector("img").unwrap();
    let episode_selector = selector(".epz").unwrap();

    for element in document.select(&venz_selector) {
        let title = text_from_or(&element, &title_selector, "");
        let href = attr_from(&element, &link_selector, "href").unwrap_or_default();
        let slug = extract_slug(&href);
        let poster = attr_from_or(&element, &img_selector, "src", "");
        let episode_count = text_from_or(&element, &episode_selector, "N/A");
        let anime_url = attr_from_or(&element, &link_selector, "href", "");

        if !title.is_empty() {
            complete_anime.push(CompleteAnimeItem {
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

pub fn parse_genres(html: &str) -> Result<Vec<Genre>, AppError> {
    let document = parse_html(html);
    let mut genres = Vec::new();
    let genre_selector = selector(".genres li a, .genre-list a").unwrap();

    for element in document.select(&genre_selector) {
        let name = text(&element);
        let url = attr(&element, "href").unwrap_or_default();
        let slug = extract_slug(&url);

        if !name.is_empty() && !slug.is_empty() {
            genres.push(Genre { name, slug, url });
        }
    }

    Ok(genres)
}

pub fn parse_anime_detail_document(html: &str) -> Result<AnimeDetailData, AppError> {
    let document = parse_html(html);

    let info_selector = selector(".infozingle p").unwrap();
    let poster_selector = selector(".fotoanime img").unwrap();
    let synopsis_selector = selector(".sinopc").unwrap();
    let genre_link_selector = selector("a").unwrap();
    let episode_list_selector = selector(".episodelist ul li a").unwrap();
    let recommendation_selector = selector("#recommend-anime-series .isi-anime").unwrap();
    let recommendation_title_selector = selector(".judul-anime a").unwrap();
    let recommendation_img_selector = selector("img").unwrap();

    let mut title = String::new();
    let mut alternative_title = String::new();
    let mut r#type: Option<String> = None;
    let mut status: Option<String> = None;
    let mut release_date = String::new();
    let mut studio = String::new();
    let producers = Vec::new();

    for element in document.select(&info_selector) {
        let text = text(&element);
        if text.contains("Judul:") {
            title = text.replace("Judul:", "").trim().to_string();
        } else if text.contains("Japanese:") {
            alternative_title = text.replace("Japanese:", "").trim().to_string();
        } else if text.contains("Type:") {
            let type_str = text.replace("Type:", "").trim().to_string();
            if !type_str.is_empty() {
                r#type = Some(type_str);
            }
        } else if text.contains("Status:") {
            let status_str = text.replace("Status:", "").trim().to_string();
            if !status_str.is_empty() {
                status = Some(status_str);
            }
        } else if text.contains("Tanggal Rilis:") {
            release_date = text.replace("Tanggal Rilis:", "").trim().to_string();
        } else if text.contains("Studio:") {
            studio = text.replace("Studio:", "").trim().to_string();
        }
    }

    let poster = document
        .select(&poster_selector)
        .next()
        .and_then(|e| e.value().attr("src"))
        .unwrap_or("")
        .to_string();

    let synopsis = text_from_or(&document.root_element(), &synopsis_selector, "");

    let mut genres = Vec::new();
    if let Some(genres_element) = document
        .select(&info_selector)
        .find(|e| text(&e).contains("Genres:"))
    {
        for genre_link in genres_element.select(&genre_link_selector) {
            let name = text(&genre_link);
            let anime_url = attr(&genre_link, "href").unwrap_or_default();
            let genre_slug = extract_slug(&anime_url);
            genres.push(DetailGenre {
                name,
                slug: genre_slug,
                anime_url,
            });
        }
    }

    let mut episode_lists = Vec::new();
    for element in document.select(&episode_list_selector) {
        let episode = text(&element);
        let href = attr(&element, "href").unwrap_or_default();
        let slug = extract_slug(&href);
        episode_lists.push(EpisodeList { episode, slug });
    }

    let mut recommendations = Vec::new();
    for element in document.select(&recommendation_selector) {
        let title = text_from_or(&element, &recommendation_title_selector, "");
        let poster = attr_from_or(&element, &recommendation_img_selector, "src", "");
        let href = element
            .select(&genre_link_selector)
            .next()
            .and_then(|e| e.value().attr("href"))
            .unwrap_or("");

        let slug = extract_slug(href);

        recommendations.push(Recommendation {
            title,
            slug,
            poster,
            status: None,
            r#type: None,
        });
    }

    Ok(AnimeDetailData {
        title,
        alternative_title,
        poster,
        r#type,
        status,
        release_date,
        studio,
        genres,
        synopsis,
        episode_lists,
        batch: vec![],
        producers,
        recommendations,
    })
}

pub fn parse_anime_page(
    html: &str,
    slug: &str,
) -> Result<(Vec<CompleteAnimeListItem>, Pagination), AppError> {
    let document = parse_html(html);
    let mut anime_list = Vec::new();

    let item_selector = selector(".venz ul li").unwrap();
    let title_selector = selector(".thumbz h2.jdlflm").unwrap();
    let link_selector = selector("a").unwrap();
    let img_selector = selector("img").unwrap();
    let episode_selector = selector(".epz").unwrap();
    let pagination_selector = selector(".pagenavix .page-numbers:not(.next)").unwrap();
    let next_selector = selector(".pagenavix .next.page-numbers").unwrap();

    for element in document.select(&item_selector) {
        let title = text_from_or(&element, &title_selector, "");
        let anime_url = attr_from_or(&element, &link_selector, "href", "");
        let slug = extract_slug(&anime_url);
        let poster = attr_from_or(&element, &img_selector, "src", "");
        let episode_count = text_from_or(&element, &episode_selector, "N/A");

        if !title.is_empty() {
            anime_list.push(CompleteAnimeListItem {
                title,
                slug,
                poster,
                episode_count,
                anime_url,
            });
        }
    }

    let current_page = slug.parse::<u32>().unwrap_or(1);
    let last_visible_page = document
        .select(&pagination_selector)
        .next_back()
        .and_then(|e| e.text().collect::<String>().trim().parse::<u32>().ok())
        .unwrap_or(1);

    let has_next_page = document.select(&next_selector).next().is_some();
    let next_page = if has_next_page {
        Some(current_page + 1)
    } else {
        None
    };
    let has_previous_page = current_page > 1;
    let previous_page = if has_previous_page {
        Some(current_page - 1)
    } else {
        None
    };

    let pagination = Pagination {
        current_page,
        last_visible_page,
        has_next_page,
        next_page,
        has_previous_page,
        previous_page,
    };

    Ok((anime_list, pagination))
}

pub fn parse_ongoing_anime_document(
    html: &str,
    slug: &str,
) -> Result<(Vec<OngoingAnimeListItem>, Pagination), AppError> {
    let document = parse_html(html);
    let mut anime_list = Vec::new();

    let item_selector = selector(".venz ul li").unwrap();
    let title_selector = selector(".thumbz h2.jdlflm").unwrap();
    let link_selector = selector("a").unwrap();
    let img_selector = selector("img").unwrap();
    let score_selector = selector(".epz").unwrap();
    let pagination_selector = selector(".pagenavix .page-numbers:not(.next)").unwrap();
    let next_selector = selector(".pagenavix .next.page-numbers").unwrap();

    for element in document.select(&item_selector) {
        let title = text_from_or(&element, &title_selector, "");
        let anime_url = attr_from_or(&element, &link_selector, "href", "");
        let slug = extract_slug(&anime_url);
        let poster = attr_from_or(&element, &img_selector, "src", "");
        let score = text_from_or(&element, &score_selector, "N/A");

        if !title.is_empty() {
            anime_list.push(OngoingAnimeListItem {
                title,
                slug,
                poster,
                score,
                anime_url,
            });
        }
    }

    let current_page = slug.parse::<u32>().unwrap_or(1);
    let last_visible_page = document
        .select(&pagination_selector)
        .next_back()
        .and_then(|e| e.text().collect::<String>().trim().parse::<u32>().ok())
        .unwrap_or(1);

    let has_next_page = document.select(&next_selector).next().is_some();
    let next_page = if has_next_page {
        Some(current_page + 1)
    } else {
        None
    };
    let has_previous_page = current_page > 1;
    let previous_page = if has_previous_page {
        Some(current_page - 1)
    } else {
        None
    };

    let pagination = Pagination {
        current_page,
        last_visible_page,
        has_next_page,
        next_page,
        has_previous_page,
        previous_page,
    };

    Ok((anime_list, pagination))
}

pub fn parse_latest_anime_document(
    html: &str,
    slug: &str,
) -> Result<(Vec<LatestAnimeItem>, Pagination), AppError> {
    let document = parse_html(html);
    let mut anime_list = Vec::new();

    let item_selector = selector(".venz ul li").unwrap();
    let title_selector = selector(".thumbz h2.jdlflm").unwrap();
    let link_selector = selector("a").unwrap();
    let img_selector = selector("img").unwrap();
    let episode_selector = selector(".epz").unwrap();
    let pagination_selector = selector(".pagenavix .page-numbers:not(.next)").unwrap();
    let next_selector = selector(".pagenavix .next.page-numbers").unwrap();

    for element in document.select(&item_selector) {
        let title = text_from_or(&element, &title_selector, "");
        let anime_url = attr_from_or(&element, &link_selector, "href", "");
        let slug = extract_slug(&anime_url);
        let poster = attr_from_or(&element, &img_selector, "src", "");
        let episode = text_from_or(&element, &episode_selector, "N/A");

        if !title.is_empty() {
            anime_list.push(LatestAnimeItem {
                title,
                slug,
                poster,
                episode,
                anime_url,
            });
        }
    }

    let current_page = slug.parse::<u32>().unwrap_or(1);
    let last_visible_page = document
        .select(&pagination_selector)
        .next_back()
        .and_then(|e| e.text().collect::<String>().trim().parse::<u32>().ok())
        .unwrap_or(1);

    let has_next_page = document.select(&next_selector).next().is_some();
    let next_page = if has_next_page {
        Some(current_page + 1)
    } else {
        None
    };
    let has_previous_page = current_page > 1;
    let previous_page = if has_previous_page {
        Some(current_page - 1)
    } else {
        None
    };

    let pagination = Pagination {
        current_page,
        last_visible_page,
        has_next_page,
        next_page,
        has_previous_page,
        previous_page,
    };

    Ok((anime_list, pagination))
}

pub fn parse_search_anime_document(
    html: &str,
    slug: &str,
) -> Result<(Vec<SearchAnimeItem>, Pagination), AppError> {
    let document = parse_html(html);
    let mut anime_list = Vec::new();

    let item_selector = selector(".venz ul li").unwrap();
    let title_selector = selector(".thumbz h2.jdlflm").unwrap();
    let link_selector = selector("a").unwrap();
    let img_selector = selector("img").unwrap();
    let episode_selector = selector(".epz").unwrap();
    let genre_selector = selector(".genre-tag").unwrap();
    let status_selector = selector(".status").unwrap();
    let rating_selector = selector(".rating").unwrap();
    let pagination_selector = selector(".pagenavix .page-numbers:not(.next)").unwrap();
    let next_selector = selector(".pagenavix .next.page-numbers").unwrap();

    for element in document.select(&item_selector) {
        let title = text_from_or(&element, &title_selector, "");
        let anime_url = attr_from_or(&element, &link_selector, "href", "");
        let slug = extract_slug(&anime_url);
        let poster = attr_from_or(&element, &img_selector, "src", "");
        let episode = text_from_or(&element, &episode_selector, "N/A");

        let mut genres = Vec::new();
        for genre_elem in element.select(&genre_selector) {
            genres.push(text(&genre_elem));
        }

        let status = text_from_or(&element, &status_selector, "");
        let rating = text_from_or(&element, &rating_selector, "");

        if !title.is_empty() {
            anime_list.push(SearchAnimeItem {
                title,
                slug,
                poster,
                episode,
                anime_url,
                genres,
                status,
                rating,
            });
        }
    }

    let current_page = slug.parse::<u32>().unwrap_or(1);
    let last_visible_page = document
        .select(&pagination_selector)
        .next_back()
        .and_then(|e| e.text().collect::<String>().trim().parse::<u32>().ok())
        .unwrap_or(1);

    let has_next_page = document.select(&next_selector).next().is_some();
    let next_page = if has_next_page {
        Some(current_page + 1)
    } else {
        None
    };
    let has_previous_page = current_page > 1;
    let previous_page = if has_previous_page {
        Some(current_page - 1)
    } else {
        None
    };

    let pagination = Pagination {
        current_page,
        last_visible_page,
        has_next_page,
        next_page,
        has_previous_page,
        previous_page,
    };

    Ok((anime_list, pagination))
}

pub fn parse_genre_anime_document(
    html: &str,
    slug: &str,
) -> Result<(Vec<GenreAnimeItem>, Pagination), AppError> {
    let document = parse_html(html);
    let mut anime_list = Vec::new();

    let item_selector = selector(".venz ul li").unwrap();
    let title_selector = selector(".thumbz h2.jdlflm").unwrap();
    let link_selector = selector("a").unwrap();
    let img_selector = selector("img").unwrap();
    let episode_selector = selector(".epz").unwrap();
    let pagination_selector = selector(".pagenavix .page-numbers:not(.next)").unwrap();
    let next_selector = selector(".pagenavix .next.page-numbers").unwrap();

    for element in document.select(&item_selector) {
        let title = text_from_or(&element, &title_selector, "");
        let anime_url = attr_from_or(&element, &link_selector, "href", "");
        let slug = extract_slug(&anime_url);
        let poster = attr_from_or(&element, &img_selector, "src", "");
        let episode = text_from_or(&element, &episode_selector, "N/A");

        if !title.is_empty() {
            anime_list.push(GenreAnimeItem {
                title,
                slug,
                poster,
                episode,
                anime_url,
            });
        }
    }

    let current_page = slug.parse::<u32>().unwrap_or(1);
    let last_visible_page = document
        .select(&pagination_selector)
        .next_back()
        .and_then(|e| e.text().collect::<String>().trim().parse::<u32>().ok())
        .unwrap_or(1);

    let has_next_page = document.select(&next_selector).next().is_some();
    let next_page = if has_next_page {
        Some(current_page + 1)
    } else {
        None
    };
    let has_previous_page = current_page > 1;
    let previous_page = if has_previous_page {
        Some(current_page - 1)
    } else {
        None
    };

    let pagination = Pagination {
        current_page,
        last_visible_page,
        has_next_page,
        next_page,
        has_previous_page,
        previous_page,
    };

    Ok((anime_list, pagination))
}

pub fn parse_anime_full_document(html: &str, slug: &str) -> Result<AnimeFullData, AppError> {
    let document = parse_html(html);

    let episode_title_selector = selector("h1.posttl").unwrap();
    let image_selector = selector(".cukder img").unwrap();
    let stream_selector = selector("#embed_holder iframe").unwrap();
    let download_item_selector = selector(".download ul li").unwrap();
    let resolution_selector = selector("strong").unwrap();
    let link_selector = selector("a").unwrap();
    let next_episode_selector = selector(".flir a[title*='Episode Selanjutnya']").unwrap();
    let previous_episode_selector = selector(".flir a[title*='Episode Sebelumnya']").unwrap();

    let episode = document
        .select(&episode_title_selector)
        .next()
        .map(|e| text(&e))
        .unwrap_or_default();

    let episode_number = episode
        .split("Episode")
        .nth(1)
        .map(|s| s.trim().to_string())
        .unwrap_or_default();

    let image_url = document
        .select(&image_selector)
        .next()
        .and_then(|e| attr(&e, "src"))
        .unwrap_or_default();

    let stream_url = document
        .select(&stream_selector)
        .next()
        .and_then(|e| attr(&e, "src"))
        .unwrap_or_default();

    let mut download_urls = std::collections::HashMap::new();

    for element in document.select(&download_item_selector) {
        let resolution = element
            .select(&resolution_selector)
            .next()
            .map(|e| text(&e))
            .unwrap_or_default();

        let mut links = Vec::new();
        for link_element in element.select(&link_selector) {
            let server = text(&link_element);
            let url = attr(&link_element, "href").unwrap_or_default();
            links.push(DownloadLink { server, url });
        }

        if !resolution.is_empty() && !links.is_empty() {
            download_urls.insert(resolution, links);
        }
    }

    let next_episode_element = document.select(&next_episode_selector).next();
    let previous_episode_element = document.select(&previous_episode_selector).next();

    let next_episode_slug = next_episode_element
        .and_then(|e| attr(&e, "href"))
        .and_then(|href| {
            href.split('/')
                .nth(href.split('/').count().saturating_sub(2))
                .map(|s| s.to_string() + "/")
        });

    let previous_episode_slug = previous_episode_element
        .and_then(|e| attr(&e, "href"))
        .and_then(|href| {
            href.split('/')
                .nth(href.split('/').count().saturating_sub(2))
                .map(|s| s.to_string() + "/")
        });

    Ok(AnimeFullData {
        episode,
        episode_number,
        anime: AnimeInfo {
            slug: slug.to_string(),
        },
        has_next_episode: next_episode_slug.is_some(),
        next_episode: next_episode_slug.map(|s| EpisodeInfo { slug: s }),
        has_previous_episode: previous_episode_slug.is_some(),
        previous_episode: previous_episode_slug.map(|s| EpisodeInfo { slug: s }),
        stream_url,
        download_urls,
        image_url,
    })
}
