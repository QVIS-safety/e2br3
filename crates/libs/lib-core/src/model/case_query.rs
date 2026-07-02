//! Dynamic case query builder (Phase 2, 2.2).
//!
//! Translates catalog-validated conditions into a parameterized SQL `WHERE`
//! clause. Identifiers (tables/columns) come only from the compiled field
//! catalog — never from client input — and all values are bound as parameters,
//! so the builder is injection-safe. The builder is a pure function and is unit
//! tested without a database.

use crate::model::case_query_catalog::{
	find_item, DataType, FieldSource, JoinKind, Operator,
};
use serde::Deserialize;
use std::fmt;

/// A raw condition as received from the client.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RawCondition {
	pub page: String,
	pub item: String,
	pub operator: Operator,
	#[serde(default)]
	pub values: Vec<String>,
}

/// A condition resolved against the catalog and checked for arity.
#[derive(Debug, Clone)]
pub struct ValidatedCondition {
	pub source: FieldSource,
	pub data_type: DataType,
	pub operator: Operator,
	pub values: Vec<String>,
}

/// Validation failure for a single condition.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QueryValidationError {
	UnknownField { page: String, item: String },
	OperatorNotAllowed { page: String, item: String, operator: Operator },
	WrongValueCount { page: String, item: String, operator: Operator, expected: &'static str, got: usize },
}

impl fmt::Display for QueryValidationError {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		match self {
			QueryValidationError::UnknownField { page, item } => {
				write!(f, "unknown query field {page}.{item}")
			}
			QueryValidationError::OperatorNotAllowed { page, item, operator } => {
				write!(f, "operator {operator:?} not allowed for {page}.{item}")
			}
			QueryValidationError::WrongValueCount { page, item, operator, expected, got } => {
				write!(
					f,
					"operator {operator:?} on {page}.{item} expects {expected} value(s), got {got}"
				)
			}
		}
	}
}

/// Number of values an operator requires.
fn expected_value_count(operator: Operator) -> &'static str {
	match operator {
		Operator::Null | Operator::NotNull => "0",
		Operator::Range => "2",
		Operator::In => "at least 1",
		_ => "1",
	}
}

fn value_count_ok(operator: Operator, got: usize) -> bool {
	match operator {
		Operator::Null | Operator::NotNull => got == 0,
		Operator::Range => got == 2,
		Operator::In => got >= 1,
		_ => got == 1,
	}
}

/// Resolves and checks each raw condition against the catalog.
pub fn validate_conditions(
	raw: &[RawCondition],
) -> Result<Vec<ValidatedCondition>, QueryValidationError> {
	raw.iter()
		.map(|cond| {
			let item = find_item(&cond.page, &cond.item).ok_or_else(|| {
				QueryValidationError::UnknownField {
					page: cond.page.clone(),
					item: cond.item.clone(),
				}
			})?;
			if !item.data_type.operators().contains(&cond.operator) {
				return Err(QueryValidationError::OperatorNotAllowed {
					page: cond.page.clone(),
					item: cond.item.clone(),
					operator: cond.operator,
				});
			}
			if !value_count_ok(cond.operator, cond.values.len()) {
				return Err(QueryValidationError::WrongValueCount {
					page: cond.page.clone(),
					item: cond.item.clone(),
					operator: cond.operator,
					expected: expected_value_count(cond.operator),
					got: cond.values.len(),
				});
			}
			Ok(ValidatedCondition {
				source: item.source,
				data_type: item.data_type,
				operator: cond.operator,
				values: cond.values.clone(),
			})
		})
		.collect()
}

/// Builds a parameterized `WHERE` clause and the ordered bind values.
///
/// Placeholders are `$1..$n`. Returns `("TRUE", [])` when there are no
/// conditions. All bound values are strings; SQL casts them to the column's
/// canonical type (`::numeric`, `::date`) so comparisons are type-correct.
pub fn build_where(conditions: &[ValidatedCondition]) -> (String, Vec<String>) {
	if conditions.is_empty() {
		return ("TRUE".to_string(), Vec::new());
	}

	let mut binds: Vec<String> = Vec::new();
	let mut predicates: Vec<String> = Vec::new();

	for cond in conditions {
		let prefix = match cond.source.join {
			JoinKind::CaseColumn => "c",
			_ => "t",
		};
		let core = build_predicate(prefix, cond, &mut binds);
		let predicate = match cond.source.join {
			JoinKind::CaseColumn => core,
			JoinKind::OneToOne(table) | JoinKind::OneToMany(table) => {
				format!(
					"EXISTS (SELECT 1 FROM {table} t WHERE t.case_id = c.id AND {core})"
				)
			}
		};
		predicates.push(predicate);
	}

	(predicates.join(" AND "), binds)
}

/// Renders the column expression and a value-placeholder generator for a data
/// type. Text-like types cast the column to text; numeric/date cast the bound
/// value to the column type.
fn build_predicate(
	prefix: &str,
	cond: &ValidatedCondition,
	binds: &mut Vec<String>,
) -> String {
	let column = cond.source.column;
	let (col_expr, val_cast): (String, &str) = match cond.data_type {
		DataType::Text | DataType::Code | DataType::Bool => {
			(format!("{prefix}.{column}::text"), "")
		}
		DataType::Integer | DataType::Decimal => {
			(format!("{prefix}.{column}"), "::numeric")
		}
		DataType::Date => (format!("{prefix}.{column}"), "::date"),
	};

	let mut next_placeholder = |binds: &mut Vec<String>, value: &str| -> String {
		binds.push(value.to_string());
		format!("${}{}", binds.len(), val_cast)
	};

	match cond.operator {
		Operator::Equal => {
			format!("{col_expr} = {}", next_placeholder(binds, &cond.values[0]))
		}
		Operator::NotEqual => {
			format!(
				"{col_expr} IS DISTINCT FROM {}",
				next_placeholder(binds, &cond.values[0])
			)
		}
		Operator::Range => {
			let lo = next_placeholder(binds, &cond.values[0]);
			let hi = next_placeholder(binds, &cond.values[1]);
			format!("{col_expr} BETWEEN {lo} AND {hi}")
		}
		Operator::Like => {
			format!(
				"{col_expr} ILIKE '%' || {} || '%'",
				next_placeholder(binds, &cond.values[0])
			)
		}
		Operator::NotLike => {
			format!(
				"{col_expr} NOT ILIKE '%' || {} || '%'",
				next_placeholder(binds, &cond.values[0])
			)
		}
		Operator::In => {
			let placeholders: Vec<String> = cond
				.values
				.iter()
				.map(|value| next_placeholder(binds, value))
				.collect();
			format!("{col_expr} IN ({})", placeholders.join(", "))
		}
		Operator::Null => format!("{col_expr} IS NULL"),
		Operator::NotNull => format!("{col_expr} IS NOT NULL"),
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	fn raw(page: &str, item: &str, operator: Operator, values: &[&str]) -> RawCondition {
		RawCondition {
			page: page.to_string(),
			item: item.to_string(),
			operator,
			values: values.iter().map(|value| value.to_string()).collect(),
		}
	}

	#[test]
	fn empty_conditions_match_all() {
		let (sql, binds) = build_where(&[]);
		assert_eq!(sql, "TRUE");
		assert!(binds.is_empty());
	}

	#[test]
	fn case_column_equal() {
		let conditions =
			validate_conditions(&[raw("CASE", "dg_prd_key", Operator::Equal, &["ABC"])])
				.unwrap();
		let (sql, binds) = build_where(&conditions);
		assert_eq!(sql, "c.dg_prd_key::text = $1");
		assert_eq!(binds, vec!["ABC".to_string()]);
	}

	#[test]
	fn one_to_many_like_uses_exists() {
		let conditions = validate_conditions(&[raw(
			"DG",
			"medicinal_product",
			Operator::Like,
			&["aspirin"],
		)])
		.unwrap();
		let (sql, binds) = build_where(&conditions);
		assert_eq!(
			sql,
			"EXISTS (SELECT 1 FROM drug_information t WHERE t.case_id = c.id AND t.medicinal_product::text ILIKE '%' || $1 || '%')"
		);
		assert_eq!(binds, vec!["aspirin".to_string()]);
	}

	#[test]
	fn numeric_range_casts_values() {
		let conditions = validate_conditions(&[raw(
			"DM",
			"age_at_time_of_onset",
			Operator::Range,
			&["18", "65"],
		)])
		.unwrap();
		let (sql, binds) = build_where(&conditions);
		assert_eq!(
			sql,
			"EXISTS (SELECT 1 FROM patient_information t WHERE t.case_id = c.id AND t.age_at_time_of_onset BETWEEN $1::numeric AND $2::numeric)"
		);
		assert_eq!(binds, vec!["18".to_string(), "65".to_string()]);
	}

	#[test]
	fn date_equal_casts_value() {
		let conditions = validate_conditions(&[raw(
			"CI",
			"date_first_received",
			Operator::Equal,
			&["2026-01-01"],
		)])
		.unwrap();
		let (sql, _) = build_where(&conditions);
		assert!(sql.contains("t.date_first_received_from_source = $1::date"));
	}

	#[test]
	fn in_operator_expands_placeholders() {
		let conditions = validate_conditions(&[raw(
			"CASE",
			"status",
			Operator::In,
			&["draft", "validated"],
		)])
		.unwrap();
		let (sql, binds) = build_where(&conditions);
		assert_eq!(sql, "c.status::text IN ($1, $2)");
		assert_eq!(binds.len(), 2);
	}

	#[test]
	fn null_operator_binds_nothing() {
		let conditions =
			validate_conditions(&[raw("CASE", "dg_prd_key", Operator::Null, &[])])
				.unwrap();
		let (sql, binds) = build_where(&conditions);
		assert_eq!(sql, "c.dg_prd_key::text IS NULL");
		assert!(binds.is_empty());
	}

	#[test]
	fn multiple_conditions_number_placeholders_sequentially() {
		let conditions = validate_conditions(&[
			raw("CASE", "dg_prd_key", Operator::Equal, &["X"]),
			raw("DG", "medicinal_product", Operator::Like, &["y"]),
		])
		.unwrap();
		let (sql, binds) = build_where(&conditions);
		assert!(sql.contains("c.dg_prd_key::text = $1"));
		assert!(sql.contains("$2 || '%'"));
		assert!(sql.contains(" AND "));
		assert_eq!(binds, vec!["X".to_string(), "y".to_string()]);
	}

	#[test]
	fn unknown_field_rejected() {
		let err = validate_conditions(&[raw("CASE", "nope", Operator::Equal, &["x"])])
			.unwrap_err();
		assert!(matches!(err, QueryValidationError::UnknownField { .. }));
	}

	#[test]
	fn operator_not_allowed_rejected() {
		// Range is invalid for a Text field.
		let err =
			validate_conditions(&[raw("CASE", "dg_prd_key", Operator::Range, &["a", "b"])])
				.unwrap_err();
		assert!(matches!(err, QueryValidationError::OperatorNotAllowed { .. }));
	}

	#[test]
	fn wrong_value_count_rejected() {
		let err =
			validate_conditions(&[raw("DM", "age_at_time_of_onset", Operator::Range, &["1"])])
				.unwrap_err();
		assert!(matches!(err, QueryValidationError::WrongValueCount { .. }));
	}
}
