use lib_core::validation::{find_canonical_rule, VALIDATION_RULES};
use serde::Serialize;

#[derive(Debug, Serialize)]
struct ValidationRuleExport {
	code: String,
	profile: String,
	section: String,
	blocking: bool,
	message: String,
	condition: String,
	export_directive: Option<String>,
}

fn main() {
	let mut out: Vec<ValidationRuleExport> = VALIDATION_RULES
		.iter()
		.map(|rule| {
			let canonical = find_canonical_rule(rule.code)
				.expect("canonical rule must exist for every metadata rule");
			ValidationRuleExport {
				code: rule.code.to_string(),
				profile: rule.profile.as_str().to_string(),
				section: rule.section.to_string(),
				blocking: rule.blocking,
				message: rule.message.to_string(),
				condition: canonical.condition.as_str().to_string(),
				export_directive: canonical
					.export_directive
					.map(|directive| directive.as_str().to_string()),
			}
		})
		.collect();
	out.sort_by(|a, b| a.code.cmp(&b.code));

	let json =
		serde_json::to_string_pretty(&out).expect("serialize validation rules");
	println!("{json}");
}
