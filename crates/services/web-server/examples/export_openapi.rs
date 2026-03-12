use utoipa::OpenApi;
use web_server::openapi::ApiDoc;

fn main() {
	let spec = ApiDoc::openapi();
	println!(
		"{}",
		serde_json::to_string_pretty(&spec).expect("serialize OpenAPI spec")
	);
}
