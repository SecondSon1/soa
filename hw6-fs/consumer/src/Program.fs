module Warehouse.Program

open Serilog

[<EntryPoint>]
let main _ =
    Log.Logger <-
        LoggerConfiguration()
            .MinimumLevel.Information()
            .WriteTo.Console(outputTemplate = "[{Timestamp:HH:mm:ss} {Level:u3}] {Message:lj}{NewLine}{Exception}")
            .CreateLogger()

    Log.Information("Starting warehouse consumer service")
    let config = Config.load ()
    use session = Cassandra.connect config
    let repo = Cassandra.Repository session
    use dlq = new Dlq.DlqProducer(config)
    let kafkaConnected = ref false
    Http.start config.HttpPort (fun () -> kafkaConnected.Value && repo.IsAvailable())
    Kafka.run config dlq kafkaConnected (Handler.handle repo)
    Log.CloseAndFlush()
    0
