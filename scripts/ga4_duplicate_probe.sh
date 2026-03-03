#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
if [ -f "${ROOT_DIR}/.env" ]; then
  set -a
  # shellcheck disable=SC1090
  source "${ROOT_DIR}/.env"
  set +a
fi

WINDOW_DAYS="${GA4_DUPLICATE_WINDOW_DAYS:-14}"
if ! [[ "${WINDOW_DAYS}" =~ ^[0-9]+$ ]] || [ "${WINDOW_DAYS}" -lt 1 ]; then
  echo "GA4_DUPLICATE_WINDOW_DAYS must be a positive integer" >&2
  exit 2
fi

json_get() {
  python3 - "$1" "$2" <<'PY'
import json
import sys
obj = json.load(open(sys.argv[1]))
val = obj.get(sys.argv[2], "")
if val is None:
    val = ""
print(val)
PY
}

creds_env_var="${ANALYTICS_GA4_READ_CREDENTIALS_ENV_VAR:-GOOGLE_APPLICATION_CREDENTIALS}"
creds_path="${!creds_env_var:-}"
if [ -z "${creds_path}" ] || [ ! -f "${creds_path}" ]; then
  echo "Missing service account json path in ${creds_env_var}" >&2
  exit 3
fi

client_email="$(json_get "${creds_path}" client_email)"
private_key="$(json_get "${creds_path}" private_key)"
token_uri="$(json_get "${creds_path}" token_uri)"
if [ -z "${token_uri}" ]; then
  token_uri="https://oauth2.googleapis.com/token"
fi

sa_project_id="$(json_get "${creds_path}" project_id)"
project_id="${ANALYTICS_GA4_BIGQUERY_PROJECT_ID:-${sa_project_id}}"
if [ -z "${project_id}" ]; then
  echo "Missing project id: set ANALYTICS_GA4_BIGQUERY_PROJECT_ID or include project_id in service account json" >&2
  exit 4
fi

property_id="${GA4_PROPERTY_ID:-}"
dataset_id="${ANALYTICS_GA4_BIGQUERY_DATASET_ID:-}"
if [ -z "${dataset_id}" ]; then
  if [ -z "${property_id}" ]; then
    echo "Missing dataset id: set ANALYTICS_GA4_BIGQUERY_DATASET_ID or GA4_PROPERTY_ID" >&2
    exit 5
  fi
  dataset_id="analytics_${property_id}"
fi

max_bytes="${ANALYTICS_GA4_BIGQUERY_MAX_BYTES_BILLED:-3000000000}"
if ! [[ "${max_bytes}" =~ ^[0-9]+$ ]] || [ "${max_bytes}" -le 0 ]; then
  echo "ANALYTICS_GA4_BIGQUERY_MAX_BYTES_BILLED must be a positive integer" >&2
  exit 6
fi

b64url() {
  openssl base64 -A | tr '+/' '-_' | tr -d '='
}

header='{"alg":"RS256","typ":"JWT"}'
now_epoch="$(date +%s)"
exp_epoch="$((now_epoch + 3600))"
claims="$(python3 - <<PY
import json
print(json.dumps({
  "iss": r'''${client_email}''',
  "sub": r'''${client_email}''',
  "scope": "https://www.googleapis.com/auth/bigquery",
  "aud": r'''${token_uri}''',
  "iat": ${now_epoch},
  "exp": ${exp_epoch}
}, separators=(",", ":")))
PY
)"
unsigned_jwt="$(printf %s "${header}" | b64url).$(printf %s "${claims}" | b64url)"
signature="$(printf %s "${unsigned_jwt}" | openssl dgst -sha256 -sign <(printf %s "${private_key}") -binary | b64url)"
assertion="${unsigned_jwt}.${signature}"

oauth_response="$(curl -sS -X POST "${token_uri}" \
  -H "Content-Type: application/x-www-form-urlencoded" \
  --data-urlencode grant_type=urn:ietf:params:oauth:grant-type:jwt-bearer \
  --data-urlencode assertion="${assertion}")"
access_token="$(python3 - <<PY
import json
obj = json.loads(r'''${oauth_response}''')
print(obj.get("access_token", ""))
PY
)"
if [ -z "${access_token}" ]; then
  echo "Failed to obtain BigQuery OAuth token" >&2
  echo "${oauth_response}" >&2
  exit 7
fi

run_query() {
  local sql="$1"
  local payload
  payload="$(python3 - <<PY
import json
print(json.dumps({
  "query": r'''${sql}''',
  "useLegacySql": False,
  "maximumBytesBilled": "${max_bytes}",
  "timeoutMs": 90000
}))
PY
)"
  curl -sS -X POST "https://bigquery.googleapis.com/bigquery/v2/projects/${project_id}/queries" \
    -H "Authorization: Bearer ${access_token}" \
    -H "Content-Type: application/json" \
    -d "${payload}"
}

overall_sql="$(cat <<SQL
WITH base AS (
  SELECT
    event_name,
    user_pseudo_id,
    event_timestamp,
    (SELECT value.int_value FROM UNNEST(event_params) WHERE key = 'ga_session_id') AS ga_session_id,
    event_bundle_sequence_id,
    batch_event_index,
    TIMESTAMP_TRUNC(TIMESTAMP_MICROS(event_timestamp), SECOND) AS event_second
  FROM \`${project_id}.${dataset_id}.events_*\`
  WHERE _TABLE_SUFFIX BETWEEN FORMAT_DATE('%Y%m%d', DATE_SUB(CURRENT_DATE(), INTERVAL ${WINDOW_DAYS} DAY))
    AND FORMAT_DATE('%Y%m%d', DATE_SUB(CURRENT_DATE(), INTERVAL 1 DAY))
), strict_sig AS (
  SELECT
    COUNT(*) AS total_rows,
    COUNT(DISTINCT TO_JSON_STRING(STRUCT(
      event_name,
      user_pseudo_id,
      event_timestamp,
      ga_session_id,
      event_bundle_sequence_id,
      batch_event_index
    ))) AS unique_rows
  FROM base
), soft_sig AS (
  SELECT
    COUNT(*) AS total_rows,
    SUM(occurrences - 1) AS extra_rows
  FROM (
    SELECT
      event_name,
      user_pseudo_id,
      ga_session_id,
      event_second,
      COUNT(*) AS occurrences
    FROM base
    GROUP BY 1,2,3,4
  )
)
SELECT
  strict_sig.total_rows,
  strict_sig.total_rows - strict_sig.unique_rows AS strict_duplicate_rows,
  SAFE_DIVIDE(strict_sig.total_rows - strict_sig.unique_rows, strict_sig.total_rows) AS strict_duplicate_ratio,
  soft_sig.extra_rows AS soft_duplicate_rows,
  SAFE_DIVIDE(soft_sig.extra_rows, soft_sig.total_rows) AS soft_duplicate_ratio
FROM strict_sig, soft_sig
SQL
)"

soft_offenders_sql="$(cat <<SQL
WITH base AS (
  SELECT
    PARSE_DATE('%Y%m%d', event_date) AS event_day,
    device.category AS device_category,
    event_name,
    user_pseudo_id,
    (SELECT value.int_value FROM UNNEST(event_params) WHERE key = 'ga_session_id') AS ga_session_id,
    TIMESTAMP_TRUNC(TIMESTAMP_MICROS(event_timestamp), SECOND) AS event_second
  FROM \`${project_id}.${dataset_id}.events_*\`
  WHERE _TABLE_SUFFIX BETWEEN FORMAT_DATE('%Y%m%d', DATE_SUB(CURRENT_DATE(), INTERVAL ${WINDOW_DAYS} DAY))
    AND FORMAT_DATE('%Y%m%d', DATE_SUB(CURRENT_DATE(), INTERVAL 1 DAY))
), total AS (
  SELECT event_day, device_category, event_name, COUNT(*) AS total_rows
  FROM base
  GROUP BY 1,2,3
), grouped AS (
  SELECT
    event_day,
    device_category,
    event_name,
    user_pseudo_id,
    ga_session_id,
    event_second,
    COUNT(*) AS occurrences
  FROM base
  GROUP BY 1,2,3,4,5,6
), duplicate_groups AS (
  SELECT
    event_day,
    device_category,
    event_name,
    COUNT(*) AS duplicate_groups,
    SUM(occurrences - 1) AS extra_rows,
    SUM(occurrences) AS grouped_rows
  FROM grouped
  WHERE occurrences > 1
  GROUP BY 1,2,3
)
SELECT
  d.event_day,
  d.device_category,
  d.event_name,
  t.total_rows,
  d.duplicate_groups,
  d.grouped_rows,
  d.extra_rows,
  SAFE_DIVIDE(d.extra_rows, t.total_rows) AS soft_duplicate_ratio
FROM duplicate_groups d
JOIN total t USING (event_day, device_category, event_name)
ORDER BY soft_duplicate_ratio DESC, d.extra_rows DESC
LIMIT 25
SQL
)"

overall_json="$(run_query "${overall_sql}")"
soft_json="$(run_query "${soft_offenders_sql}")"

overall_file="$(mktemp)"
soft_file="$(mktemp)"
trap 'rm -f "${overall_file}" "${soft_file}"' EXIT
printf '%s' "${overall_json}" >"${overall_file}"
printf '%s' "${soft_json}" >"${soft_file}"

GA4_DUP_WINDOW_DAYS="${WINDOW_DAYS}" \
GA4_DUP_PROJECT_ID="${project_id}" \
GA4_DUP_DATASET_ID="${dataset_id}" \
python3 - "${overall_file}" "${soft_file}" <<'PY'
import json
import os
import sys

def load(path):
    with open(path, "r", encoding="utf-8") as f:
        return json.load(f)

def parse_row_vals(row):
    return [item.get("v") for item in row.get("f", [])]

overall = load(sys.argv[1])
soft = load(sys.argv[2])

if "error" in overall:
    print(json.dumps({"overall_error": overall["error"]}, indent=2))
    raise SystemExit(8)
if "error" in soft:
    print(json.dumps({"soft_error": soft["error"]}, indent=2))
    raise SystemExit(9)

overall_rows = overall.get("rows") or []
if not overall_rows:
    print(json.dumps({"message": "No GA4 rows in requested window."}, indent=2))
    raise SystemExit(0)

vals = parse_row_vals(overall_rows[0])
summary = {
    "total_rows": int(vals[0]) if vals[0] is not None else 0,
    "strict_duplicate_rows": int(vals[1]) if vals[1] is not None else 0,
    "strict_duplicate_ratio": float(vals[2]) if vals[2] is not None else 0.0,
    "soft_duplicate_rows": int(vals[3]) if vals[3] is not None else 0,
    "soft_duplicate_ratio": float(vals[4]) if vals[4] is not None else 0.0,
}

soft_rows = []
for row in soft.get("rows", []):
    vals = parse_row_vals(row)
    soft_rows.append(
        {
            "event_day": vals[0],
            "device_category": vals[1],
            "event_name": vals[2],
            "total_rows": int(vals[3]) if vals[3] is not None else 0,
            "duplicate_groups": int(vals[4]) if vals[4] is not None else 0,
            "grouped_rows": int(vals[5]) if vals[5] is not None else 0,
            "extra_rows": int(vals[6]) if vals[6] is not None else 0,
            "soft_duplicate_ratio": float(vals[7]) if vals[7] is not None else 0.0,
        }
    )

print(
    json.dumps(
        {
            "window_days": int(os.environ.get("GA4_DUP_WINDOW_DAYS", "0")),
            "project_id": os.environ.get("GA4_DUP_PROJECT_ID", ""),
            "dataset_id": os.environ.get("GA4_DUP_DATASET_ID", ""),
            "summary": summary,
            "top_soft_duplicate_offenders": soft_rows,
        },
        indent=2,
    )
)
PY
