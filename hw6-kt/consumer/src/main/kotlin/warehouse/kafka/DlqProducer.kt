package warehouse.kafka

import com.fasterxml.jackson.module.kotlin.jacksonObjectMapper
import org.apache.kafka.clients.producer.KafkaProducer
import org.apache.kafka.clients.producer.ProducerConfig
import org.apache.kafka.clients.producer.ProducerRecord
import org.apache.kafka.common.serialization.StringSerializer
import org.slf4j.LoggerFactory
import warehouse.config.Config
import java.time.Instant
import java.util.Properties

class DlqProducer(private val config: Config) {
    private val log = LoggerFactory.getLogger(DlqProducer::class.java)
    private val mapper = jacksonObjectMapper()

    private val producer: KafkaProducer<String, String>

    init {
        val props = Properties().apply {
            put(ProducerConfig.BOOTSTRAP_SERVERS_CONFIG, config.kafkaBootstrapServers)
            put(ProducerConfig.KEY_SERIALIZER_CLASS_CONFIG, StringSerializer::class.java.name)
            put(ProducerConfig.VALUE_SERIALIZER_CLASS_CONFIG, StringSerializer::class.java.name)
        }
        producer = KafkaProducer(props)
    }

    fun send(originalEvent: Any, error: Throwable, partition: Int, offset: Long) {
        val dlqMessage = mapOf(
            "original_event" to originalEvent.toString(),
            "error_reason" to (error.message ?: "Unknown error"),
            "error_code" to "VALIDATION_ERROR",
            "failed_at" to Instant.now().toString(),
            "kafka_metadata" to mapOf(
                "partition" to partition,
                "offset" to offset,
            ),
        )

        val record = ProducerRecord<String, String>(config.dlqTopic, mapper.writeValueAsString(dlqMessage))
        try {
            producer.send(record).get()
        } catch (e: Exception) {
            log.error("dlq send failed: {}", e.message)
        }
    }

    fun close() {
        producer.close()
    }
}
