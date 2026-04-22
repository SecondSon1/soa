#!/usr/bin/env fish

set -l script_name (basename (status --current-filename))

if test (count $argv) -gt 2
    echo "Usage: $script_name [topic] [consumer-group]"
    exit 1
end

set -l topic movie-events
set -l group clickhouse-consumer

if test (count $argv) -ge 1
    set topic $argv[1]
end

if test (count $argv) -eq 2
    set group $argv[2]
end

cd (dirname (status --current-filename))/..

echo "# Topic: $topic"
docker compose exec -T kafka1 kafka-topics \
    --describe \
    --topic $topic \
    --bootstrap-server kafka1:29092
or exit $status

echo
echo "# Consumer group: $group"
docker compose exec -T kafka1 kafka-consumer-groups \
    --describe \
    --group $group \
    --bootstrap-server kafka1:29092

