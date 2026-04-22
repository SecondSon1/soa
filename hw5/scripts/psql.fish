#!/usr/bin/env fish

set -l script_name (basename (status --current-filename))

if test (count $argv) -gt 2
    echo "Usage: $script_name [database] [sql]"
    exit 1
end

set -l database cinema
set -l sql

if test (count $argv) -ge 1
    set database $argv[1]
end

if test (count $argv) -eq 2
    set sql $argv[2]
end

cd (dirname (status --current-filename))/..

if test -n "$sql"
    docker compose exec -T postgres psql -U cinema -d $database -c "$sql"
else
    docker compose exec postgres psql -U cinema -d $database
end

