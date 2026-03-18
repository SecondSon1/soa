package com.example.flight.grpc.auth

import io.grpc.Metadata
import io.grpc.ServerCall
import io.grpc.ServerCallHandler
import io.grpc.ServerInterceptor
import io.grpc.Status

private const val GRPC_API_KEY_HEADER = "x-api-key"

class ApiKeyServerInterceptor(
    private val expectedApiKey: String,
) : ServerInterceptor {
    private val apiKeyMetadataKey = Metadata.Key.of(GRPC_API_KEY_HEADER, Metadata.ASCII_STRING_MARSHALLER)

    override fun <ReqT : Any?, RespT : Any?> interceptCall(
        call: ServerCall<ReqT, RespT>,
        headers: Metadata,
        next: ServerCallHandler<ReqT, RespT>,
    ): ServerCall.Listener<ReqT> {
        val providedApiKey = headers.get(apiKeyMetadataKey)
        if (providedApiKey != expectedApiKey) {
            call.close(Status.UNAUTHENTICATED.withDescription("Missing or invalid API key"), Metadata())
            return object : ServerCall.Listener<ReqT>() {}
        }

        return next.startCall(call, headers)
    }
}
