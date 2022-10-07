#!/bin/bash
# This script bootstraps a docker container containing a single-node mongodb
# replica set, needed for running the backend test suite.
# Usage: './test-setup.sh' (no arguments)

# Constants.
readonly TESTDB_IMAGE='dreip-backend-testdb'
readonly TESTDB_CONTAINER='dreip-backend-testdb'
readonly TESTDB_PASSWORD='password'

# Exit with an intelligent message when any command fails.
last_command=""
trap 'last_command=$BASH_COMMAND' DEBUG
trap $'echo "Command \'$last_command\' failed with exit code $?"' ERR
set -e

# Manually die with message.
die() {
  echo "$*"
  exit 1
}

# Do our best to make the working directory the location of this script.
# Strip filename off script path. If that leaves us with something that looks
# like a directory, go to it.
script_path="${BASH_SOURCE[0]%/*}"
if [[ -n "$script_path" && "$script_path" != "$0" ]]; then
  cd "$script_path"
fi

# Check we have access to docker.
echo -n "Checking docker... "
which docker &>/dev/null || die "Docker could not be found; is it on the path?"
docker ps &>/dev/null || die "Docker could not be reached; is it running and do we have permission?"
echo "docker available"

# Get rid of the container if it already exists.
name_matches=$(docker ps -a --filter name="^$TESTDB_CONTAINER$" | wc -l)
if [[ "$name_matches" -gt 1 ]]; then
  # We always get one header line. If there are more lines, then the container exists.
  echo "Container $TESTDB_CONTAINER exists and will be destroyed."
  read -rp "Continue? [y/N]: " response
  if [[ $response != "y" && $response != "Y" ]]; then
    echo "Aborting script"
    exit 2
  fi
  # Destroy it.
  echo -n "Destroying $TESTDB_CONTAINER... "
  docker rm --force "$TESTDB_CONTAINER" >/dev/null
  echo "done"
fi

# Build the image.
echo -n "Rebuilding DB image... "
docker build -t "$TESTDB_IMAGE" ./db >/dev/null
echo "done"

# Run the container.
echo -n "Launching container... "
docker run --rm -d \
  --env "MONGODB_REPLICA_SET_MODE=primary" \
  --env "MONGODB_REPLICA_SET_KEY=$TESTDB_PASSWORD" \
  --env "MONGODB_ROOT_PASSWORD=$TESTDB_PASSWORD" \
  --name "$TESTDB_CONTAINER" \
  "$TESTDB_IMAGE" >/dev/null
echo "done"

# Get the IP.
echo -n "Getting IP address... "
ip_addr=$(docker inspect --format '{{range .NetworkSettings.Networks}}{{.IPAddress}}{{end}}' \
          "$TESTDB_CONTAINER")
mongo_uri="mongodb://root:$TESTDB_PASSWORD@$ip_addr"
echo "done"
echo "Container $TESTDB_CONTAINER running at $ip_addr"

# Configure the replica set hostname.
# This defaults to the container hostname (container ID), but must be routable
# from this host, so set it to the container IP.
# This only works once mongodb is fully up and ready, which is difficult to
# detect exactly, so wait for the port to be available and then retry in a loop.
echo -n "Configuring replica set... "
./db/wait-for-it.sh "$ip_addr:27017" &>/dev/null
remaining_attempts=10
set +e
while [[ $remaining_attempts -gt 0 ]]; do
  ((remaining_attempts--))
  if [[ $remaining_attempts -eq 0 ]]; then
    # Die if the last attempt fails.
    set -e
  fi
  mongosh "$mongo_uri" \
    --eval "cfg = rs.conf(); cfg.members[0].host = '$ip_addr'; rs.reconfig(cfg)" \
    &>/dev/null && break
  sleep 1
done
echo "done"

echo
echo "Please set 'export ROCKET_DB_URI=$mongo_uri' in your shell"
echo "Then run tests with 'cargo test [--all-features]'"