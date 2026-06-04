package warehouse.config

data class Config(
    val kafkaBootstrapServers: String,
    val schemaRegistryUrl: String,
    val kafkaTopic: String,
    val kafkaGroupId: String,
    val cassandraContactPoints: String,
    val cassandraPort: Int,
    val cassandraDatacenter: String,
    val cassandraKeyspace: String,
    val dlqTopic: String,
    val httpPort: Int,
) {
    companion object {
        fun fromEnv(): Config = Config(
            kafkaBootstrapServers = env("KAFKA_BOOTSTRAP_SERVERS", "localhost:9092"),
            schemaRegistryUrl = env("SCHEMA_REGISTRY_URL", "http://localhost:8081"),
            kafkaTopic = env("KAFKA_TOPIC", "warehouse-events"),
            kafkaGroupId = env("KAFKA_GROUP_ID", "warehouse-state-consumer"),
            dlqTopic = env("KAFKA_DLQ_TOPIC", "warehouse-events-dlq"),
            cassandraContactPoints = env("CASSANDRA_CONTACT_POINTS", "localhost"),
            cassandraPort = env("CASSANDRA_PORT", "9042").toInt(),
            cassandraDatacenter = env("CASSANDRA_DATACENTER", "datacenter1"),
            cassandraKeyspace = env("CASSANDRA_KEYSPACE", "warehouse"),
            httpPort = env("HTTP_PORT", "8080").toInt(),
        )

        private fun env(key: String, default: String): String =
            System.getenv(key) ?: default
    }
}
