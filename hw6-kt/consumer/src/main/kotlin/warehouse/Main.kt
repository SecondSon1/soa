package warehouse

import warehouse.cassandra.Client
import warehouse.cassandra.Repository
import warehouse.config.Config
import warehouse.handler.EventHandler
import warehouse.http.Server
import warehouse.kafka.Consumer
import warehouse.kafka.DlqProducer

fun main() {
    val config = Config.fromEnv()
    val session = Client(config).connect()
    val repository = Repository(session)
    val handler = EventHandler(repository)
    val dlqProducer = DlqProducer(config)
    val consumer = Consumer(config, handler, dlqProducer)

    val httpServer = Server(config.httpPort) {
        consumer.isConnected() && repository.isAvailable()
    }
    httpServer.start()

    Runtime.getRuntime().addShutdownHook(Thread {
        consumer.shutdown()
        httpServer.stop()
        dlqProducer.close()
        session.close()
    })

    consumer.run()
}
