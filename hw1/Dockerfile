FROM golang:1.25.7-alpine3.23 AS build
WORKDIR /src
COPY go.mod ./
RUN go mod download
COPY src ./src
RUN CGO_ENABLED=0 go build -o /out/app ./src

FROM alpine:3.23
RUN adduser -D -u 10001 appuser
USER appuser
COPY --from=build /out/app /app
EXPOSE 8080
ENTRYPOINT ["/app"]
