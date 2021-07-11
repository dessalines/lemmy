#!/bin/bash
set -e

export LEMMY_TEST_SEND_SYNC=1
export RUST_BACKTRACE=1
export RUST_LOG="warn,lemmy_server=debug,lemmy_api=debug,lemmy_api_common=debug,lemmy_api_crud=debug,lemmy_apub=debug,lemmy_apub_receive=debug,lemmy_db_queries=debug,lemmy_db_schema=debug,lemmy_db_views=debug,lemmy_db_views_actor=debug,lemmy_db_views_moderator=debug,lemmy_routes=debug,lemmy_utils=debug,lemmy_websocket=debug"

for INSTANCE in lemmy_alpha lemmy_beta lemmy_gamma lemmy_delta lemmy_epsilon; do
  psql "${LEMMY_DATABASE_URL}/lemmy" -c "DROP DATABASE IF EXISTS $INSTANCE"
  psql "${LEMMY_DATABASE_URL}/lemmy" -c "CREATE DATABASE $INSTANCE"
done

if [ -z "$DO_WRITE_HOSTS_FILE" ]; then
  if ! grep -q lemmy-alpha /etc/hosts; then
    echo "Please add the following to your /etc/hosts file, then press enter:

      127.0.0.1       lemmy-alpha
      127.0.0.1       lemmy-beta
      127.0.0.1       lemmy-gamma
      127.0.0.1       lemmy-delta
      127.0.0.1       lemmy-epsilon"
    read -p ""
  fi
else
  for INSTANCE in lemmy-alpha lemmy-beta lemmy-gamma lemmy-delta lemmy-epsilon; do
    echo "127.0.0.1 $INSTANCE" >> /etc/hosts
  done
fi

killall lemmy_server || true

echo "$PWD"

echo "start alpha"
LEMMY_HOSTNAME=lemmy-alpha:8541 \
  LEMMY_CONFIG_LOCATION=./docker/federation/lemmy_alpha.hjson \
  LEMMY_DATABASE_URL="${LEMMY_DATABASE_URL}/lemmy_alpha" \
  LEMMY_HOSTNAME="lemmy-alpha:8541" \
  target/lemmy_server >/tmp/lemmy_alpha.out 2>&1 &

echo "start beta"
LEMMY_HOSTNAME=lemmy-beta:8551 \
  LEMMY_CONFIG_LOCATION=./docker/federation/lemmy_beta.hjson \
  LEMMY_DATABASE_URL="${LEMMY_DATABASE_URL}/lemmy_beta" \
  target/lemmy_server >/tmp/lemmy_beta.out 2>&1 &

echo "start gamma"
LEMMY_HOSTNAME=lemmy-gamma:8561 \
  LEMMY_CONFIG_LOCATION=./docker/federation/lemmy_gamma.hjson \
  LEMMY_DATABASE_URL="${LEMMY_DATABASE_URL}/lemmy_gamma" \
  target/lemmy_server >/tmp/lemmy_gamma.out 2>&1 &

echo "start delta"
# An instance with only an allowlist for beta
LEMMY_HOSTNAME=lemmy-delta:8571 \
  LEMMY_CONFIG_LOCATION=./docker/federation/lemmy_delta.hjson \
  LEMMY_DATABASE_URL="${LEMMY_DATABASE_URL}/lemmy_delta" \
  target/lemmy_server >/tmp/lemmy_delta.out 2>&1 &

echo "start epsilon"
# An instance who has a blocklist, with lemmy-alpha blocked
LEMMY_HOSTNAME=lemmy-epsilon:8581 \
  LEMMY_CONFIG_LOCATION=./docker/federation/lemmy_epsilon.hjson \
  LEMMY_DATABASE_URL="${LEMMY_DATABASE_URL}/lemmy_epsilon" \
  target/lemmy_server >/tmp/lemmy_epsilon.out 2>&1 &

echo "wait for all instances to start"
while [[ "$(curl -s -o /dev/null -w '%{http_code}' 'localhost:8541/api/v3/site')" != "200" ]]; do sleep 1; done
while [[ "$(curl -s -o /dev/null -w '%{http_code}' 'localhost:8551/api/v3/site')" != "200" ]]; do sleep 1; done
while [[ "$(curl -s -o /dev/null -w '%{http_code}' 'localhost:8561/api/v3/site')" != "200" ]]; do sleep 1; done
while [[ "$(curl -s -o /dev/null -w '%{http_code}' 'localhost:8571/api/v3/site')" != "200" ]]; do sleep 1; done
while [[ "$(curl -s -o /dev/null -w '%{http_code}' 'localhost:8581/api/v3/site')" != "200" ]]; do sleep 1; done
