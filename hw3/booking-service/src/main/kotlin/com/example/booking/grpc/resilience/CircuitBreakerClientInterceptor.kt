package com.example.booking.grpc.resilience

import io.grpc.ClientCall
import io.grpc.ClientInterceptor
import io.grpc.ForwardingClientCall
import io.grpc.ForwardingClientCallListener
import io.grpc.Metadata
import io.grpc.MethodDescriptor
import io.grpc.Status

class CircuitBreakerClientInterceptor(
    private val circuitBreaker: GrpcCircuitBreaker,
) : ClientInterceptor {
    override fun <ReqT : Any?, RespT : Any?> interceptCall(
        method: MethodDescriptor<ReqT, RespT>,
        callOptions: io.grpc.CallOptions,
        next: io.grpc.Channel,
    ): ClientCall<ReqT, RespT> {
        val methodName = method.bareMethodName()
        if (!circuitBreaker.tryAcquirePermission(methodName)) {
            return ShortCircuitClientCall(circuitBreaker.rejectionStatus())
        }

        val call = next.newCall(method, callOptions)
        return object : ForwardingClientCall.SimpleForwardingClientCall<ReqT, RespT>(call) {
            override fun start(
                responseListener: Listener<RespT>,
                headers: Metadata,
            ) {
                val listener =
                    object : ForwardingClientCallListener.SimpleForwardingClientCallListener<RespT>(responseListener) {
                        override fun onClose(
                            status: Status,
                            trailers: Metadata,
                        ) {
                            circuitBreaker.onCallFinished(methodName, status)
                            super.onClose(status, trailers)
                        }
                    }
                super.start(listener, headers)
            }
        }
    }
}

private class ShortCircuitClientCall<ReqT, RespT>(
    private val status: Status,
) : ClientCall<ReqT, RespT>() {
    override fun start(
        responseListener: Listener<RespT>,
        headers: Metadata,
    ) {
        responseListener.onClose(status, Metadata())
    }

    override fun request(numMessages: Int) = Unit

    override fun cancel(
        message: String?,
        cause: Throwable?,
    ) = Unit

    override fun halfClose() = Unit

    override fun sendMessage(message: ReqT) = Unit
}

private fun <ReqT, RespT> MethodDescriptor<ReqT, RespT>.bareMethodName(): String =
    fullMethodName.substringAfterLast('/')
