#!/usr/bin/env fish

set -l script_name (basename (status --current-filename))

if test (count $argv) -gt 1
    echo "Usage: $script_name [prefix]"
    exit 1
end

set -l path local/movie-analytics

if test (count $argv) -eq 1
    set path "$path/$argv[1]"
end

cd (dirname (status --current-filename))/..

docker compose run --rm --entrypoint /bin/sh minio-init -lc \
    "mc alias set local http://minio:9000 minioadmin minioadmin >/dev/null && mc ls --recursive $path"
