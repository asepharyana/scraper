use axum::Router;
use std::sync::Arc;

use crate::shared::state::AppState;

pub mod anime;
pub mod anime2;
pub mod komik;
pub mod proxy;

pub fn routes(router: Router<Arc<AppState>>) -> Router<Arc<AppState>> {
    let router = anime::route::routes(router);
    let router = anime2::route::routes(router);
    let router = komik::route::routes(router);
    let router = proxy::route::routes(router);
    router
}
