use std::env;

use axum::Router;
use axum_extra::routing::RouterExt;
use deadpool_diesel::postgres::Manager;
use deadpool_diesel::postgres::Pool;
use deadpool_diesel::Runtime;
use dotenvy::dotenv;
use hypermedia_systems_rust::api;
use hypermedia_systems_rust::html_views;
use hypermedia_systems_rust::AppState;
use tower_http::services::ServeDir;

// TODO:
// - [ ] try using `serde(try_from = "...")` with contacts and user facing contacts.
//   want to report multiple errors and for errors to be user-facing
// - [ ] test with playwright
// - [ ] test with forms (in the style of zero to prod in rust)
// - [ ] shift to use mvc (with state extract for contacts as model?)
// - [x] provide extract for hx-trigger id
// - [ ] add macro for hx-trigger id
// - [x] separate out database from html_views
//       - Can wait to do this later, rust-analyzer makes it easy to extract to variable and then to function.
// - [ ] style with tailwind
//       - https://www.crocodile.dev/blog/css-transitions-with-tailwind-and-htmx
//       - https://tailwindcss.com/docs/plugins#adding-variants
// - [ ] (maybe) move away from dotenvy to just using `.envrc`
//       - would that impact deploying or testing?
fn establish_connection() -> Pool {
    dotenv().ok();
    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let manager = Manager::new(&database_url, Runtime::Tokio1);
    Pool::builder(manager)
        .max_size(8)
        .build()
        .unwrap_or_else(|_| panic!("Error connecting to {}", database_url))
}

#[tokio::main]
async fn main() {
    let pool = establish_connection();
    let starting_state = AppState {
        db_pool: pool,
        flash_config: axum_flash::Config::new(axum_flash::Key::generate()),
    };
    let api_routes = Router::new()
        .typed_get(api::get_contacts)
        .typed_get(api::get_contact)
        .typed_put(api::update_contact)
        .typed_delete(api::delete_contact)
        .typed_post(api::new_contact);

    let app = Router::new()
        .typed_get(html_views::root)
        .typed_get(html_views::contacts)
        .typed_get(html_views::contacts_new_get)
        .typed_get(html_views::contacts_view)
        .typed_get(html_views::contacts_count)
        .typed_get(html_views::contacts_edit_get)
        .typed_get(html_views::contacts_email_get)
        .typed_post(html_views::contacts_new_post)
        .typed_post(html_views::contacts_edit_post)
        .typed_delete(html_views::contacts_delete)
        .typed_delete(html_views::contacts_delete_all)
        .nest("/api/v1", api_routes)
        .with_state(starting_state)
        .nest_service("/dist", ServeDir::new("dist"));

    #[cfg(debug_assertions)]
    use axum::extract::Request;
    #[cfg(debug_assertions)]
    fn not_htmx_predicate<T>(req: &Request<T>) -> bool {
        !req.headers().contains_key("hx-request")
    }

    #[cfg(debug_assertions)]
    let app =
        app.layer(tower_livereload::LiveReloadLayer::new().request_predicate(not_htmx_predicate));

    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000")
        .await
        .unwrap();
    println!("{}", listener.local_addr().unwrap());
    axum::serve(listener, app).await.unwrap();
}
