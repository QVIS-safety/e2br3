use super::*;

#[derive(Debug, Default)]
pub(super) struct ReconcileRuntimeStore {
	last_run_at: Option<OffsetDateTime>,
	last_success_at: Option<OffsetDateTime>,
	last_error: Option<String>,
	total_runs: u64,
	total_errors: u64,
	total_attempted: u64,
	total_succeeded: u64,
	total_failed: u64,
	total_skipped: u64,
}

pub(super) fn reconcile_runtime_store() -> &'static Mutex<ReconcileRuntimeStore> {
	static STORE: OnceLock<Mutex<ReconcileRuntimeStore>> = OnceLock::new();
	STORE.get_or_init(|| Mutex::new(ReconcileRuntimeStore::default()))
}

pub(super) fn record_reconcile_result(result: &SubmissionReconcileResult) {
	let now = OffsetDateTime::now_utc();
	let mut store = reconcile_runtime_store()
		.lock()
		.expect("reconcile runtime stats lock");
	store.last_run_at = Some(now);
	store.last_success_at = Some(now);
	store.last_error = None;
	store.total_runs = store.total_runs.saturating_add(1);
	store.total_attempted = store
		.total_attempted
		.saturating_add(result.attempted as u64);
	store.total_succeeded = store
		.total_succeeded
		.saturating_add(result.succeeded as u64);
	store.total_failed = store.total_failed.saturating_add(result.failed as u64);
	store.total_skipped = store.total_skipped.saturating_add(result.skipped as u64);
}

pub(super) fn record_reconcile_error(err: &str) {
	let now = OffsetDateTime::now_utc();
	let mut store = reconcile_runtime_store()
		.lock()
		.expect("reconcile runtime stats lock");
	store.last_run_at = Some(now);
	store.last_error = Some(err.to_string());
	store.total_runs = store.total_runs.saturating_add(1);
	store.total_errors = store.total_errors.saturating_add(1);
}

pub fn get_reconcile_runtime_status() -> SubmissionReconcileRuntimeStatus {
	let store = reconcile_runtime_store()
		.lock()
		.expect("reconcile runtime stats lock");
	SubmissionReconcileRuntimeStatus {
		last_run_at: store.last_run_at,
		last_success_at: store.last_success_at,
		last_error: store.last_error.clone(),
		total_runs: store.total_runs,
		total_errors: store.total_errors,
		total_attempted: store.total_attempted,
		total_succeeded: store.total_succeeded,
		total_failed: store.total_failed,
		total_skipped: store.total_skipped,
	}
}
