module Warehouse.Http

open System.Net
open System.Text
open Prometheus
open Serilog

let start (port: int) (healthCheck: unit -> bool) =
    let listener = new HttpListener()
    listener.Prefixes.Add(sprintf "http://+:%d/" port)
    listener.Start()
    Log.Information("HTTP server started on port {Port}", port)

    let rec loop () =
        async {
            let! context = listener.GetContextAsync() |> Async.AwaitTask
            let resp = context.Response

            try
                match context.Request.Url.AbsolutePath with
                | "/health" ->
                    let ok = healthCheck ()
                    resp.StatusCode <- if ok then 200 else 503
                    resp.ContentType <- "application/json"
                    let body = if ok then """{"status":"UP"}""" else """{"status":"DOWN"}"""
                    let bytes = Encoding.UTF8.GetBytes(body)
                    resp.OutputStream.Write(bytes, 0, bytes.Length)

                | "/metrics" ->
                    resp.StatusCode <- 200
                    resp.ContentType <- "text/plain; version=0.0.4; charset=utf-8"

                    Metrics
                        .DefaultRegistry
                        .CollectAndExportAsTextAsync(resp.OutputStream)
                        .GetAwaiter()
                        .GetResult()

                | _ -> resp.StatusCode <- 404
            with ex ->
                resp.StatusCode <- 500
                Log.Error(ex, "HTTP request error")

            resp.Close()
            return! loop ()
        }

    loop () |> Async.Start
