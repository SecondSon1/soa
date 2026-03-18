FROM gradle:9.4.0-jdk25 AS build

WORKDIR /workspace
COPY . .
RUN gradle --no-daemon :booking-service:installDist :flight-service:installDist

FROM eclipse-temurin:25-jre AS booking-service

WORKDIR /app
COPY --from=build /workspace/booking-service/build/install/booking-service /app

ENV PORT=8080
EXPOSE 8080

CMD ["/app/bin/booking-service"]

FROM eclipse-temurin:25-jre AS flight-service

WORKDIR /app
COPY --from=build /workspace/flight-service/build/install/flight-service /app

ENV PORT=9090
EXPOSE 9090

CMD ["/app/bin/flight-service"]
