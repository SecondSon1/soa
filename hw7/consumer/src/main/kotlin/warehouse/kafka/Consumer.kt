package warehouse.kafka

import io.confluent.kafka.serializers.KafkaAvroDeserializer
import io.confluent.kafka.serializers.KafkaAvroDeserializerConfig
import org.apache.kafka.clients.consumer.ConsumerConfig
import org.apache.kafka.clients.consumer.KafkaConsumer
import org.apache.kafka.common.errors.WakeupException
import org.apache.kafka.common.serialization.StringDeserializer
import org.slf4j.LoggerFactory
import warehouse.config.Config
import warehouse.handler.EventHandler
import warehouse.metrics.Metrics
import warehouse.model.WarehouseEvent
import java.time.Duration
import java.util.Properties

class Consumer(
    private val config: Config,
    private val handler: EventHandler,
    private val dlq: DlqProducer,
) {
    private val log = LoggerFactory.getLogger(Consumer::class.java)

    @Volatile
    private var connected = false

    val kafkaConsumer: KafkaConsumer<String, warehouse.avro.WarehouseEvent>

    init {
        val props = Properties().apply {
            put(ConsumerConfig.BOOTSTRAP_SERVERS_CONFIG, config.kafkaBootstrapServers)
            put(ConsumerConfig.GROUP_ID_CONFIG, config.kafkaGroupId)
            put(ConsumerConfig.KEY_DESERIALIZER_CLASS_CONFIG, StringDeserializer::class.java.name)
            put(ConsumerConfig.VALUE_DESERIALIZER_CLASS_CONFIG, KafkaAvroDeserializer::class.java.name)
            put(ConsumerConfig.ENABLE_AUTO_COMMIT_CONFIG, "false")
            put(ConsumerConfig.AUTO_OFFSET_RESET_CONFIG, "earliest")
            put("schema.registry.url", config.schemaRegistryUrl)
            put(KafkaAvroDeserializerConfig.SPECIFIC_AVRO_READER_CONFIG, "true")
        }
        kafkaConsumer = KafkaConsumer(props)
    }

    fun run() {
        kafkaConsumer.subscribe(listOf(config.kafkaTopic))
        log.info("subscribed to {}", config.kafkaTopic)

        try {
            while (true) {
                val records = kafkaConsumer.poll(Duration.ofMillis(1000))
                connected = true
                updateLagMetrics()
                if (records.isEmpty) continue

                for (record in records) {
                    try {
                        val event = WarehouseEvent.fromAvro(record.value())
                        log.info("event_id={} type={} p={} o={}",
                            event.eventId, event.eventType, record.partition(), record.offset())
                        handler.handle(event)
                    } catch (e: Exception) {
                        log.error("failed p={} o={}: {}", record.partition(), record.offset(), e.message)
                        dlq.send(record.value(), e, record.partition(), record.offset())
                    }
                }

                kafkaConsumer.commitSync()
            }
        } catch (_: WakeupException) {
        } finally {
            kafkaConsumer.close()
        }
    }

    private fun updateLagMetrics() {
        try {
            val assignment = kafkaConsumer.assignment()
            if (assignment.isEmpty()) return
            val endOffsets = kafkaConsumer.endOffsets(assignment)
            for (tp in assignment) {
                val committed = kafkaConsumer.committed(setOf(tp))[tp]?.offset() ?: 0
                val end = endOffsets[tp] ?: 0
                Metrics.consumerLag.labels(tp.partition().toString()).set((end - committed).toDouble())
            }
        } catch (_: Exception) {}
    }

    fun isConnected(): Boolean = connected

    fun shutdown() {
        kafkaConsumer.wakeup()
    }
}
