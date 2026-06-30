//! Narrative template rendering.
//!
//! INFO > Narrative records hold an "Auto Narrative" template containing
//! `{E2B.CODE}` tokens (UI spec example: `"{D.2.2a}세의 {D.5} 환자"`). At case
//! authoring time the tokens are replaced with the case's E2B(R3) values to
//! produce the narrative text.
//!
//! This module is the pure rendering core: it tokenizes a template and
//! substitutes values via a caller-supplied resolver (code -> value). The
//! resolver, which maps an E2B code to a concrete case value, is wired in by
//! the case service.

/// Render `template`, replacing each `{code}` token with `resolve(code)`.
///
/// * A token is `{` + content + `}`. The trimmed content is passed to `resolve`.
/// * If `resolve` returns `Some(v)`, the token is replaced with `v`.
/// * If `resolve` returns `None`, the original `{code}` token is left intact so
///   the author can see which value is missing.
/// * An unclosed `{` (no matching `}`) is emitted literally.
pub fn render_template(
	template: &str,
	resolve: impl Fn(&str) -> Option<String>,
) -> String {
	let mut out = String::with_capacity(template.len());
	let mut rest = template;
	while let Some(open) = rest.find('{') {
		out.push_str(&rest[..open]);
		let after_open = &rest[open + 1..];
		match after_open.find('}') {
			Some(close) => {
				let raw = &after_open[..close];
				let code = raw.trim();
				match resolve(code) {
					Some(value) => out.push_str(&value),
					None => {
						out.push('{');
						out.push_str(raw);
						out.push('}');
					}
				}
				rest = &after_open[close + 1..];
			}
			None => {
				// Unclosed brace: emit the rest literally and stop.
				out.push('{');
				out.push_str(after_open);
				rest = "";
			}
		}
	}
	out.push_str(rest);
	out
}

/// Collect the distinct token codes referenced by a template, in order of first
/// appearance. Useful for validating a template against available case fields.
pub fn template_tokens(template: &str) -> Vec<String> {
	let mut tokens = Vec::new();
	let mut rest = template;
	while let Some(open) = rest.find('{') {
		let after_open = &rest[open + 1..];
		match after_open.find('}') {
			Some(close) => {
				let code = after_open[..close].trim().to_string();
				if !code.is_empty() && !tokens.contains(&code) {
					tokens.push(code);
				}
				rest = &after_open[close + 1..];
			}
			None => break,
		}
	}
	tokens
}

#[cfg(test)]
mod tests {
	use super::*;
	use std::collections::HashMap;

	fn resolver(pairs: &[(&str, &str)]) -> impl Fn(&str) -> Option<String> {
		let map: HashMap<String, String> = pairs
			.iter()
			.map(|(k, v)| (k.to_string(), v.to_string()))
			.collect();
		move |code: &str| map.get(code).cloned()
	}

	#[test]
	fn renders_spec_example() {
		let r = resolver(&[("D.2.2a", "30"), ("D.5", "여성")]);
		assert_eq!(
			render_template("{D.2.2a}세의 {D.5} 환자", r),
			"30세의 여성 환자"
		);
	}

	#[test]
	fn unresolved_token_is_left_intact() {
		let r = resolver(&[("D.5", "여성")]);
		assert_eq!(
			render_template("{D.2.2a}세의 {D.5}", r),
			"{D.2.2a}세의 여성"
		);
	}

	#[test]
	fn trims_whitespace_inside_braces() {
		let r = resolver(&[("D.5", "여성")]);
		assert_eq!(render_template("{ D.5 }", r), "여성");
	}

	#[test]
	fn text_without_tokens_is_unchanged() {
		let r = resolver(&[]);
		assert_eq!(render_template("no tokens here", r), "no tokens here");
	}

	#[test]
	fn unclosed_brace_is_literal() {
		let r = resolver(&[("D.5", "여성")]);
		assert_eq!(render_template("{D.5} and {oops", r), "여성 and {oops");
	}

	#[test]
	fn template_tokens_are_distinct_and_ordered() {
		assert_eq!(
			template_tokens("{D.2.2a} {D.5} {D.2.2a}"),
			vec!["D.2.2a".to_string(), "D.5".to_string()]
		);
	}
}
