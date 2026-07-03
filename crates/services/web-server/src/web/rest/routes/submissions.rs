use axum::routing::get;
use axum::Router;
use lib_core::model::ModelManager;

use crate::web::rest::submission_rest;

/// Routes for /api/submissions
pub fn routes_submissions(mm: ModelManager) -> Router {
	Router::new()
		.route(
			"/submissions/history",
			get(submission_rest::list_all_submission_history),
		)
		.route(
			"/submissions/receiver-options",
			get(submission_rest::list_receiver_options),
		)
		.route(
			"/submissions/{id}",
			get(submission_rest::get_case_submission),
		)
		.route(
			"/submissions/{id}/events",
			get(submission_rest::list_submission_event_history),
		)
		.route(
			"/submissions/{id}/acks/{level}/download",
			get(submission_rest::download_submission_ack_text),
		)
		.route(
			"/submissions/{id}/dispatch-state",
			get(submission_rest::get_submission_dispatch_state_view),
		)
		.route(
			"/submissions/{id}/acks/mock",
			axum::routing::post(submission_rest::post_mock_ack),
		)
		.with_state(mm)
}

/// Routes for /internal/submissions (gateway callbacks)
/// Routes for /internal/submissions (gateway callbacks)
pub fn routes_submissions_internal(mm: ModelManager) -> Router {
	Router::new()
		.route(
			"/submissions/callbacks/ack",
			axum::routing::post(submission_rest::post_gateway_ack_callback),
		)
		.route(
			"/submissions/reconcile",
			axum::routing::post(submission_rest::post_reconcile_due_submissions),
		)
		.route(
			"/submissions/reconcile/status",
			get(submission_rest::get_reconcile_status),
		)
		.with_state(mm)
}
