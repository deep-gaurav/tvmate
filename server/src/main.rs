use app::*;
use axum::{
    body::Body,
    extract::{FromRef, Request, State},
    response::{IntoResponse, Response},
    routing::{get, post},
    Router,
};
use common::{endpoints, RoomProvider};
use fileserv::file_and_error_handler;
use leptos::*;
use leptos_axum::{generate_route_list, handle_server_fns_with_context, LeptosRoutes};
use leptos_router::RouteListing;
use logging::warn;
use room::{host_room, join_room};
use tower_http::compression::CompressionLayer;
use tracing::{info, level_filters::LevelFilter};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

pub mod fileserv;
pub mod room;

#[derive(FromRef, Clone)]
pub struct AppState {
    leptos_options: LeptosOptions,
    routes: Vec<RouteListing>,
    pub rooms: RoomProvider,
}

#[tokio::main]
async fn main() {
    let subscriber = tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer())
        .with(
            EnvFilter::builder()
                .with_default_directive(LevelFilter::INFO.into())
                .from_env_lossy(),
        );
    if cfg!(target_os = "linux") {
        match tracing_journald::layer() {
            Ok(journald_layer) => {
                if let Err(err) = subscriber.with(journald_layer).try_init() {
                    warn!("Cannot initialize tracing {err:#?}")
                }
            }

            Err(err) => {
                warn!("Cant get journald_layer {err:#?}");
                if let Err(err) = subscriber.try_init() {
                    warn!("Cannot initialize tracing {err:#?}")
                }
            }
        }
    } else {
        subscriber.init();
    }

    // Setting get_configuration(None) means we'll be using cargo-leptos's env values
    // For deployment these variables are:
    // <https://github.com/leptos-rs/start-axum#executing-a-server-on-a-remote-machine-without-the-toolchain>
    // Alternately a file can be specified such as Some("Cargo.toml")
    // The file would need to be included with the executable when moved to deployment
    let conf = get_configuration(Some("Cargo.toml")).await.unwrap();
    let leptos_options = conf.leptos_options;
    let addr = leptos_options.site_addr;
    let routes = generate_route_list(App);

    let compression = CompressionLayer::new();

    let app_state = AppState {
        leptos_options,
        routes: routes.clone(),
        rooms: RoomProvider::new(),
    };
    // build our application with a route
    let app = Router::new()
        .route(
            "/api/*fn_name",
            get(server_fn_handler).post(server_fn_handler),
        )
        .leptos_routes_with_handler(routes, get(leptos_routes_handler))
        .route(endpoints::HOST_ROOM, get(host_room))
        .route(endpoints::JOIN_ROOM, get(join_room))
        .fallback(file_and_error_handler)
        .layer(compression)
        .with_state(app_state);

    // run our app with hyper
    // `axum::Server` is a re-export of `hyper::Server`
    info!("listening on http://{}", &addr);
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app.into_make_service())
        .await
        .unwrap();
}

async fn leptos_routes_handler(
    State(app_state): State<AppState>,
    request: Request<Body>,
) -> Response {
    let handler = leptos_axum::render_route_with_context(
        app_state.leptos_options.clone(),
        app_state.routes.clone(),
        || (),
        App,
    );
    handler(request).await.into_response()
}

async fn server_fn_handler(
    State(_app_state): State<AppState>,
    request: Request<Body>,
) -> impl IntoResponse {
    handle_server_fns_with_context(|| {}, request).await
}
