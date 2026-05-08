module Warehouse.Config

type Config =
    { KafkaBootstrapServers: string
      SchemaRegistryUrl: string
      KafkaTopic: string
      KafkaGroupId: string
      DlqTopic: string
      CassandraContactPoints: string
      CassandraPort: int
      CassandraDatacenter: string
      CassandraKeyspace: string
      HttpPort: int }

let private env key defaultValue =
    match System.Environment.GetEnvironmentVariable key with
    | null | "" -> defaultValue
    | v -> v

let load () =
    { KafkaBootstrapServers = env "KAFKA_BOOTSTRAP_SERVERS" "localhost:9092"
      SchemaRegistryUrl = env "SCHEMA_REGISTRY_URL" "http://localhost:8081"
      KafkaTopic = env "KAFKA_TOPIC" "warehouse-events"
      KafkaGroupId = env "KAFKA_GROUP_ID" "warehouse-state-consumer"
      DlqTopic = env "KAFKA_DLQ_TOPIC" "warehouse-events-dlq"
      CassandraContactPoints = env "CASSANDRA_CONTACT_POINTS" "localhost"
      CassandraPort = env "CASSANDRA_PORT" "9042" |> int
      CassandraDatacenter = env "CASSANDRA_DATACENTER" "datacenter1"
      CassandraKeyspace = env "CASSANDRA_KEYSPACE" "warehouse"
      HttpPort = env "HTTP_PORT" "8080" |> int }
