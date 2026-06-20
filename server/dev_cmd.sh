#!/bin/sh
set -e

export KMS_KEY_ID="$(cat '/shared/cloudfront_kms_key_id')"
echo "KMS_KEY_ID=${CLOUDFRONT_KMS_KEY_ID}"

silvie-server
