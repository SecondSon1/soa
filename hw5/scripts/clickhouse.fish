#!/usr/bin/env fish

set -l script_name (basename (status --current-filename))

if test (count $argv) -gt 1
    echo "Usage: $script_name [sql]"
    exit 1
end

cd (dirname (status --current-filename))/..

if test (count $argv) -eq 1
    docker compose exec -T clickhouse clickhouse-client -q "$argv[1]"
else
    docker compose exec clickhouse clickhouse-client
end

