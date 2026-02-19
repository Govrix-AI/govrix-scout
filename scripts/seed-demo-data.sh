#!/usr/bin/env bash
# Seed demo data for Govrix Platform (requires a running server at localhost:4001)
set -euo pipefail

BASE="http://localhost:4001/api/v1"

echo "Seeding demo tenants..."
curl -s -X POST "$BASE/tenants" -H "Content-Type: application/json" -d '{"name":"acme-corp"}' | jq .
curl -s -X POST "$BASE/tenants" -H "Content-Type: application/json" -d '{"name":"beta-co"}' | jq .

echo "Loading demo policy rules..."
curl -s -X POST "$BASE/policies/reload" -H "Content-Type: application/json" -d '{
  "rules_yaml": "- name: block-pii-exfil\n  enabled: true\n  conditions:\n    - field: compliance_tag\n      operator: contains\n      value: pii\n  action: block\n- name: alert-high-cost\n  enabled: true\n  conditions:\n    - field: cost_usd\n      operator: greater_than\n      value: \"0.10\"\n  action: alert\n"
}' | jq .

echo "Done. Visit http://localhost:4001/api/v1/tenants to verify."
