#!/usr/bin/env fish

set -l script_name (basename (status --current-filename))

if test (count $argv) -gt 2
    echo "Usage: $script_name [user_count] [days_back]"
    exit 1
end

set -l user_count 20
set -l days_back 9

if test (count $argv) -ge 1
    set user_count $argv[1]
end

if test (count $argv) -eq 2
    set days_back $argv[2]
end

cd (dirname (status --current-filename))/..

set -l payload "{\"user_count\":$user_count,\"days_back\":$days_back}"

echo "POST http://localhost:3000/generate"
echo "Payload: $payload"

curl -sS -X POST http://localhost:3000/generate \
    -H "Content-Type: application/json" \
    -d "$payload"

echo
