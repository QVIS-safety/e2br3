#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd -P)"
asset_ref="${E2BR3_XML_ASSET_REF:-698f907f3abe6a555594931ace82e9fbcb9c204d}"
required_assets=(
	"docs/exporter/fda/FAERS2022Scenario1.xml"
	"docs/exporter/fda/FAERS2022Scenario2.xml"
	"docs/exporter/fda/FAERS2022Scenario3.xml"
	"docs/exporter/fda/FAERS2022Scenario5-1.xml"
	"docs/exporter/fda/FAERS2022Scenario5-2.xml"
	"docs/exporter/fda/FAERS2022Scenario5-3.xml"
	"docs/exporter/fda/FAERS2022Scenario6.xml"
	"docs/exporter/fda/FAERS2022Scenario7.xml"
	"docs/exporter/schema/multicacheschemas/MCCI_IN200100UV01.xsd"
)

xml_assets_ready() {
	local asset
	for asset in "${required_assets[@]}"; do
		[[ -f "$repo_root/$asset" ]] || return 1
	done
}

if xml_assets_ready && [[ "${E2BR3_XML_ASSET_FORCE:-0}" != "1" ]]; then
	printf 'XML validation assets already prepared\n'
	exit 0
fi

printf 'Fetching XML validation assets (%s)\n' "$asset_ref"
git -C "$repo_root" fetch --no-tags origin "$asset_ref"

mkdir -p "$repo_root/docs/exporter/schema" "$repo_root/docs/exporter/fda"
git -C "$repo_root" archive "$asset_ref" -- deploy/ec2/schemas |
	tar -x -C "$repo_root/docs/exporter/schema" --strip-components=3
git -C "$repo_root" archive "$asset_ref" -- docs/refs/instances |
	tar -x -C "$repo_root/docs/exporter/fda" --strip-components=3

if ! xml_assets_ready; then
	for asset in "${required_assets[@]}"; do
		if [[ ! -f "$repo_root/$asset" ]]; then
			printf 'required XML validation asset is missing: %s\n' "$asset" >&2
		fi
	done
	exit 1
fi
printf 'XML validation assets ready\n'
