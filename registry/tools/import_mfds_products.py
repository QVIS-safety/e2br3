#!/usr/bin/env python3
from __future__ import annotations

import argparse
import json
import math
import os
from pathlib import Path
import subprocess
import tempfile
from typing import Any, Callable
import urllib.parse
import urllib.request


API_ENDPOINT = (
    "https://apis.data.go.kr/1471000/DrugPrdtPrmsnInfoService07/"
    "getDrugPrdtPrmsnInq07"
)
PRODUCT_FIELDS = {
    "ITEM_SEQ": "item_seq",
    "ITEM_NAME": "product_name_kr",
    "ITEM_ENG_NAME": "product_name_en",
    "ENTP_NAME": "manufacturer_name_kr",
    "ENTP_ENG_NAME": "manufacturer_name_en",
    "ITEM_PERMIT_DATE": "permit_date",
    "CANCEL_DATE": "cancel_date",
    "CANCEL_NAME": "cancel_name",
}
FetchPage = Callable[[int, int, str], bytes]


def service_key_from_environment() -> str:
    key = os.environ.get("DATA_GO_KR_SERVICE_KEY", "").strip()
    if not key:
        raise ValueError("DATA_GO_KR_SERVICE_KEY is required")
    return key


def fetch_page(page: int, rows: int, service_key: str) -> bytes:
    query = urllib.parse.urlencode(
        {
            "serviceKey": service_key,
            "pageNo": page,
            "numOfRows": rows,
            "type": "json",
        }
    )
    request = urllib.request.Request(
        f"{API_ENDPOINT}?{query}",
        headers={"Accept": "application/json"},
    )
    with urllib.request.urlopen(request, timeout=60) as response:
        return response.read()


def _response_parts(raw: bytes) -> tuple[dict[str, Any], dict[str, Any]]:
    try:
        payload = json.loads(raw)
    except (UnicodeDecodeError, json.JSONDecodeError) as exc:
        raise ValueError(f"MFDS API returned invalid JSON: {exc}") from exc
    if not isinstance(payload, dict):
        raise ValueError("MFDS API response must be an object")
    response = payload.get("response", payload)
    if not isinstance(response, dict):
        raise ValueError("MFDS API response wrapper must be an object")
    header = response.get("header", {})
    body = response.get("body", {})
    if not isinstance(header, dict) or not isinstance(body, dict):
        raise ValueError("MFDS API header and body must be objects")
    result_code = str(header.get("resultCode", ""))
    if result_code not in {"00", "0"}:
        message = str(header.get("resultMsg", "unknown error"))
        raise ValueError(f"MFDS API error {result_code}: {message}")
    return header, body


def _body_items(body: dict[str, Any]) -> list[dict[str, Any]]:
    items: Any = body.get("items", [])
    if isinstance(items, dict):
        items = items.get("item", [])
    if items is None:
        return []
    if not isinstance(items, list) or not all(isinstance(item, dict) for item in items):
        raise ValueError("MFDS API body.items must be an array of objects")
    return items


def _normalize_product(item: dict[str, Any]) -> dict[str, str | None]:
    normalized: dict[str, str | None] = {}
    for source, target in PRODUCT_FIELDS.items():
        raw_value = item.get(source)
        value = "" if raw_value is None else str(raw_value).strip()
        normalized[target] = value or None
    item_seq = normalized["item_seq"]
    product_name = normalized["product_name_kr"]
    if not item_seq or not product_name:
        raise ValueError("MFDS product requires ITEM_SEQ and ITEM_NAME")
    if len(item_seq) > 10:
        raise ValueError(f"MFDS ITEM_SEQ exceeds 10 characters: {item_seq}")
    return normalized


def collect_products(
    version: str,
    service_key: str,
    *,
    fetch: FetchPage = fetch_page,
    rows_per_page: int = 500,
) -> tuple[dict[str, Any], list[tuple[int, bytes]]]:
    if not version.strip():
        raise ValueError("version must be non-empty")
    if not 1 <= rows_per_page <= 500:
        raise ValueError("rows_per_page must be between 1 and 500")

    products: dict[str, dict[str, str | None]] = {}
    raw_pages: list[tuple[int, bytes]] = []
    page = 1
    total_pages: int | None = None
    while total_pages is None or page <= total_pages:
        raw = fetch(page, rows_per_page, service_key)
        raw_pages.append((page, raw))
        _, body = _response_parts(raw)
        try:
            total_count = int(body.get("totalCount", 0))
        except (TypeError, ValueError) as exc:
            raise ValueError("MFDS API totalCount must be an integer") from exc
        if total_count < 0:
            raise ValueError("MFDS API totalCount must not be negative")
        if total_pages is None:
            total_pages = max(1, math.ceil(total_count / rows_per_page))

        for item in _body_items(body):
            product = _normalize_product(item)
            item_seq = str(product["item_seq"])
            existing = products.get(item_seq)
            if existing is not None and existing != product:
                raise ValueError(f"conflicting ITEM_SEQ {item_seq}")
            products[item_seq] = product
        page += 1

    artifact = {
        "dictionary": "mfds_product",
        "version": version.strip(),
        "language": "ko",
        "source": API_ENDPOINT,
        "products": [products[key] for key in sorted(products)],
    }
    if not artifact["products"]:
        raise ValueError("MFDS API returned no products")
    return artifact, raw_pages


def _atomic_write(path: Path, data: bytes) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    with tempfile.NamedTemporaryFile(
        mode="wb", dir=path.parent, prefix=f".{path.name}.", suffix=".tmp", delete=False
    ) as temporary:
        temporary.write(data)
        temporary.flush()
        os.fsync(temporary.fileno())
        temporary_path = Path(temporary.name)
    try:
        os.replace(temporary_path, path)
    finally:
        temporary_path.unlink(missing_ok=True)


def collect_to_paths(
    *,
    version: str,
    service_key: str,
    output: Path,
    raw_dir: Path,
    fetch: FetchPage = fetch_page,
    rows_per_page: int = 500,
) -> dict[str, Any]:
    artifact, raw_pages = collect_products(
        version, service_key, fetch=fetch, rows_per_page=rows_per_page
    )
    for page, raw in raw_pages:
        _atomic_write(raw_dir / f"page-{page:05}.json", raw)
    normalized = (json.dumps(artifact, ensure_ascii=True, indent=2) + "\n").encode()
    _atomic_write(output, normalized)
    return artifact


def main() -> int:
    parser = argparse.ArgumentParser(
        description="Collect MFDS product ITEM_SEQ terminology and optionally stage it"
    )
    parser.add_argument("--version", required=True)
    parser.add_argument("--output", type=Path)
    parser.add_argument("--raw-dir", type=Path)
    parser.add_argument("--rows-per-page", type=int, default=500)
    parser.add_argument("--collect-only", action="store_true")
    args = parser.parse_args()

    output = args.output or Path(
        f"tmp/mfds-products/mfds-products-{args.version}.json"
    )
    raw_dir = args.raw_dir or Path("tmp/mfds-products/raw") / args.version
    collect_to_paths(
        version=args.version,
        service_key=service_key_from_environment(),
        output=output,
        raw_dir=raw_dir,
        rows_per_page=args.rows_per_page,
    )
    print(f"Collected MFDS products: {output}")

    if not args.collect_only:
        subprocess.run(
            [
                "cargo",
                "run",
                "-p",
                "terminology-loader",
                "--",
                "mfds-products",
                "--input",
                str(output),
                "--version",
                args.version,
            ],
            check=True,
        )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
