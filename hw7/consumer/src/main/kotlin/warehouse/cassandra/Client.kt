package warehouse.cassandra

import com.datastax.oss.driver.api.core.CqlSession
import com.datastax.oss.driver.api.core.config.DefaultDriverOption
import com.datastax.oss.driver.api.core.config.DriverConfigLoader
import warehouse.config.Config
import java.net.InetSocketAddress

class Client(private val config: Config) {
    fun connect(): CqlSession {
        val contactPoints = config.cassandraContactPoints.split(",").map { host ->
            InetSocketAddress(host.trim(), config.cassandraPort)
        }

        return CqlSession.builder()
            .addContactPoints(contactPoints)
            .withLocalDatacenter(config.cassandraDatacenter)
            .withKeyspace(config.cassandraKeyspace)
            .withConfigLoader(
                DriverConfigLoader.programmaticBuilder()
                    .withString(DefaultDriverOption.REQUEST_CONSISTENCY, "QUORUM")
                    .build()
            )
            .build()
    }
}
