package com.example.booking.grpc.auth

import io.grpc.Metadata
import io.grpc.stub.MetadataUtils

const val GRPC_API_KEY_HEADER = "x-api-key"

fun apiKeyMetadataInterceptor(apiKey: String) =
    MetadataUtils.newAttachHeadersInterceptor(
        Metadata().apply {
            put(Metadata.Key.of(GRPC_API_KEY_HEADER, Metadata.ASCII_STRING_MARSHALLER), apiKey)
        },
    )
