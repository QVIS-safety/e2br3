//! Receiver report-due classification and due-date calculation.
//!
//! Implements the INFO > Receiver timeline rules from the UI specification
//! (QVIS Safety Database UI Spec, INFO section). A case's report-due category
//! is derived from C.1.3 (Type of Report) and the seriousness criteria
//! (E.i.3.2a-f), then the matching receiver timeline day-count is added to
//! C.1.5 (Date of Most Recent Information) to produce the report-due date.

use sqlx::types::time::Date;
use time::Duration;

/// The four report-due categories.
///
/// "Spontaneous" maps to non-solicited receiver timelines, "Solicited" to
/// solicited timelines. C.1.3 == "2" (report from study) is solicited.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReportCategory {
	NonSaeSpontaneous,
	SaeSpontaneous,
	NonSaeSolicited,
	SaeSolicited,
}

/// Classify a case into a report-due category.
///
/// * `report_type_c1_3` - value of C.1.3 (`Some("2")` = report from study).
/// * `is_serious` - true when any of E.i.3.2a-f seriousness criteria is true;
///   false when all are nullFlavor NI (non-serious).
pub fn classify_report(report_type_c1_3: Option<&str>, is_serious: bool) -> ReportCategory {
	let solicited = report_type_c1_3 == Some("2");
	match (solicited, is_serious) {
		(false, false) => ReportCategory::NonSaeSpontaneous,
		(false, true) => ReportCategory::SaeSpontaneous,
		(true, false) => ReportCategory::NonSaeSolicited,
		(true, true) => ReportCategory::SaeSolicited,
	}
}

/// A receiver's report-due day counts, one per category.
///
/// `None` means "not applicable" (the receiver does not track that category, or
/// the toggle is off) and yields no due date. Built by the caller from a
/// `ReceiverPresave` (mapping its solicited/non-solicited day counts and
/// not-applicable flags).
#[derive(Debug, Clone, Copy, Default)]
pub struct ReceiverTimeline {
	pub nsae_spontaneous: Option<i32>,
	pub sae_spontaneous: Option<i32>,
	pub nsae_solicited: Option<i32>,
	pub sae_solicited: Option<i32>,
}

impl ReceiverTimeline {
	/// The day count configured for the given category, if applicable.
	pub fn day_count(&self, category: ReportCategory) -> Option<i32> {
		match category {
			ReportCategory::NonSaeSpontaneous => self.nsae_spontaneous,
			ReportCategory::SaeSpontaneous => self.sae_spontaneous,
			ReportCategory::NonSaeSolicited => self.nsae_solicited,
			ReportCategory::SaeSolicited => self.sae_solicited,
		}
	}
}

/// Compute the report-due date: C.1.5 + the receiver's day count for the
/// case's category. Returns `None` when the category is not applicable for the
/// receiver. Negative day counts are rejected (treated as not applicable).
pub fn report_due_date(
	most_recent_info_c1_5: Date,
	timeline: &ReceiverTimeline,
	category: ReportCategory,
) -> Option<Date> {
	let days = timeline.day_count(category)?;
	if days < 0 {
		return None;
	}
	Some(most_recent_info_c1_5 + Duration::days(days as i64))
}

#[cfg(test)]
mod tests {
	use super::*;
	use time::Month;

	fn date(y: i32, m: u8, d: u8) -> Date {
		Date::from_calendar_date(y, Month::try_from(m).unwrap(), d).unwrap()
	}

	#[test]
	fn classify_spontaneous_vs_solicited_and_seriousness() {
		assert_eq!(
			classify_report(Some("1"), false),
			ReportCategory::NonSaeSpontaneous
		);
		assert_eq!(
			classify_report(None, true),
			ReportCategory::SaeSpontaneous
		);
		assert_eq!(
			classify_report(Some("2"), false),
			ReportCategory::NonSaeSolicited
		);
		assert_eq!(
			classify_report(Some("2"), true),
			ReportCategory::SaeSolicited
		);
	}

	#[test]
	fn due_date_adds_day_count_to_c1_5() {
		let tl = ReceiverTimeline {
			sae_spontaneous: Some(15),
			..Default::default()
		};
		assert_eq!(
			report_due_date(date(2026, 6, 1), &tl, ReportCategory::SaeSpontaneous),
			Some(date(2026, 6, 16))
		);
	}

	#[test]
	fn due_date_is_none_when_category_not_applicable() {
		let tl = ReceiverTimeline {
			sae_spontaneous: Some(15),
			..Default::default()
		};
		// nsae_spontaneous is None (not applicable) -> no due date
		assert_eq!(
			report_due_date(date(2026, 6, 1), &tl, ReportCategory::NonSaeSpontaneous),
			None
		);
	}

	#[test]
	fn negative_day_count_yields_no_due_date() {
		let tl = ReceiverTimeline {
			sae_solicited: Some(-5),
			..Default::default()
		};
		assert_eq!(
			report_due_date(date(2026, 6, 1), &tl, ReportCategory::SaeSolicited),
			None
		);
	}
}
